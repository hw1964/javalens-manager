use crate::config::{
    current_timestamp_string, display_path, ManagerSettings, UpdatePolicy,
};
use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};
use tar::Archive;
use walkdir::WalkDir;
use zip::ZipArchive;

const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/pzalutski-pixel/javalens-mcp/releases/latest";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedRuntimeRecord {
    pub version: String,
    pub install_dir: String,
    pub jar_path: String,
    pub asset_name: String,
    pub installed_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ReleaseStatusKind {
    Ready,
    Missing,
    UpdateAvailable,
    CheckFailed,
    CheckingDisabled,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseStatus {
    pub kind: ReleaseStatusKind,
    pub latest_version: Option<String>,
    pub default_version: Option<String>,
    pub checked_at: Option<String>,
    pub update_available: bool,
    pub detail: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    published_at: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Clone)]
struct RemoteRelease {
    version: String,
    asset_name: String,
    download_url: String,
    published_at: Option<String>,
    archive_kind: ArchiveKind,
}

#[derive(Debug, Clone, Copy)]
enum ArchiveKind {
    TarGz,
    Zip,
}

pub struct ReleaseManager {
    client: Client,
}

impl ReleaseManager {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .user_agent("javalens-manager/0.1.0")
            .build()
            .map_err(|error| format!("failed to create release manager HTTP client: {error}"))?;

        Ok(Self { client })
    }

    pub fn sync_with_settings(
        &self,
        settings: &mut ManagerSettings,
    ) -> Result<(Vec<ManagedRuntimeRecord>, ReleaseStatus), String> {
        let mut installed = self.list_installed_runtimes(settings)?;

        if !settings.auto_check_for_updates {
            let status = ReleaseStatus {
                kind: if installed.is_empty() {
                    ReleaseStatusKind::Missing
                } else {
                    ReleaseStatusKind::CheckingDisabled
                },
                latest_version: settings.last_seen_latest_version.clone(),
                default_version: settings.default_managed_runtime_version.clone(),
                checked_at: settings.last_release_check.clone(),
                update_available: false,
                detail: "Automatic JavaLens release checks are disabled.".into(),
            };
            return Ok((installed, status));
        }

        match self.fetch_latest_release() {
            Ok(release) => {
                let checked_at = current_timestamp_string();
                settings.last_release_check = Some(checked_at.clone());
                settings.last_seen_latest_version = Some(release.version.clone());

                let latest_installed = installed
                    .iter()
                    .any(|runtime| runtime.version == release.version);
                let should_download = installed.is_empty()
                    || (settings.update_policy == UpdatePolicy::Always && !latest_installed);

                let mut detail = if installed.is_empty() {
                    "No managed JavaLens runtime is cached yet.".to_string()
                } else {
                    format!("Latest upstream release is {}.", release.version)
                };

                if should_download {
                    let runtime = self.install_release(&release, settings)?;
                    installed = self.list_installed_runtimes(settings)?;
                    settings.default_managed_runtime_version = Some(runtime.version.clone());
                    detail = if installed.len() == 1 {
                        format!(
                            "Downloaded JavaLens {} into the managed tools cache.",
                            runtime.version
                        )
                    } else {
                        format!(
                            "Managed runtime {} is ready. Installed versions: {}.",
                            runtime.version,
                            installed
                                .iter()
                                .map(|item| item.version.clone())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    };
                } else if settings.default_managed_runtime_version.is_none() {
                    settings.default_managed_runtime_version =
                        installed.first().map(|runtime| runtime.version.clone());
                }

                let status = self.build_release_status(
                    Some(&release),
                    &installed,
                    settings,
                    Some(detail),
                    None,
                );

                Ok((installed, status))
            }
            Err(error) => {
                let status = self.build_release_status(
                    None,
                    &installed,
                    settings,
                    None,
                    Some(format!(
                        "Could not check the latest JavaLens release: {error}"
                    )),
                );
                Ok((installed, status))
            }
        }
    }

    pub fn download_latest_runtime(
        &self,
        settings: &mut ManagerSettings,
    ) -> Result<ManagedRuntimeRecord, String> {
        let release = self.fetch_latest_release()?;
        let runtime = self.install_release(&release, settings)?;
        settings.last_release_check = Some(current_timestamp_string());
        settings.last_seen_latest_version = Some(release.version.clone());
        settings.default_managed_runtime_version = Some(runtime.version.clone());
        Ok(runtime)
    }

    pub fn list_installed_runtimes(&self, settings: &ManagerSettings) -> Result<Vec<ManagedRuntimeRecord>, String> {
        let tools_dir = PathBuf::from(&settings.tools_dir);
        fs::create_dir_all(&tools_dir).map_err(|error| {
            format!("failed to create tools dir {}: {error}", tools_dir.display())
        })?;

        let mut runtimes = Vec::new();
        for entry in fs::read_dir(&tools_dir).map_err(|error| {
            format!(
                "failed to read tools dir {}: {error}",
                tools_dir.display()
            )
        })? {
            let entry = entry
                .map_err(|error| format!("failed to inspect managed runtime entry: {error}"))?;
            let manifest_path = entry.path().join("runtime.json");
            if manifest_path.exists() {
                let contents = fs::read_to_string(&manifest_path).map_err(|error| {
                    format!(
                        "failed to read managed runtime manifest {}: {error}",
                        manifest_path.display()
                    )
                })?;
                let runtime =
                    serde_json::from_str::<ManagedRuntimeRecord>(&contents).map_err(|error| {
                        format!(
                            "failed to parse managed runtime manifest {}: {error}",
                            manifest_path.display()
                        )
                    })?;
                runtimes.push(runtime);
            }
        }

        runtimes.sort_by(compare_runtime_versions_desc);
        Ok(runtimes)
    }

    fn fetch_latest_release(&self) -> Result<RemoteRelease, String> {
        let response = self
            .client
            .get(LATEST_RELEASE_URL)
            .header("Accept", "application/vnd.github+json")
            .send()
            .map_err(|error| format!("failed to reach GitHub releases API: {error}"))?
            .error_for_status()
            .map_err(|error| format!("GitHub releases API returned an error: {error}"))?;

        let release = response
            .json::<GitHubRelease>()
            .map_err(|error| format!("failed to parse GitHub release payload: {error}"))?;
        let version = normalize_version(&release.tag_name);

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name.ends_with(".tar.gz"))
            .or_else(|| {
                release
                    .assets
                    .iter()
                    .find(|asset| asset.name.ends_with(".zip"))
            })
            .ok_or("latest JavaLens release did not include a downloadable archive")?;

        let archive_kind = if asset.name.ends_with(".tar.gz") {
            ArchiveKind::TarGz
        } else {
            ArchiveKind::Zip
        };

        Ok(RemoteRelease {
            version,
            asset_name: asset.name.clone(),
            download_url: asset.browser_download_url.clone(),
            published_at: release.published_at,
            archive_kind,
        })
    }

    fn install_release(&self, release: &RemoteRelease, settings: &ManagerSettings) -> Result<ManagedRuntimeRecord, String> {
        let tools_dir = PathBuf::from(&settings.tools_dir);
        fs::create_dir_all(&tools_dir).map_err(|error| {
            format!("failed to create tools dir {}: {error}", tools_dir.display())
        })?;
        let target_dir = tools_dir.join(format!("javalens-{}", release.version));
        let manifest_path = target_dir.join("runtime.json");

        if manifest_path.exists() {
            let contents = fs::read_to_string(&manifest_path).map_err(|error| {
                format!(
                    "failed to read cached managed runtime manifest {}: {error}",
                    manifest_path.display()
                )
            })?;
            let runtime =
                serde_json::from_str::<ManagedRuntimeRecord>(&contents).map_err(|error| {
                    format!(
                        "failed to parse cached managed runtime manifest {}: {error}",
                        manifest_path.display()
                    )
                })?;
            return Ok(runtime);
        }

        let bytes = self
            .client
            .get(&release.download_url)
            .send()
            .map_err(|error| format!("failed to download JavaLens release archive: {error}"))?
            .error_for_status()
            .map_err(|error| format!("JavaLens archive download failed: {error}"))?
            .bytes()
            .map_err(|error| format!("failed to read JavaLens archive bytes: {error}"))?;

        let tmp_dir = tools_dir.join(format!(
            ".tmp-{}-{}",
            release.version,
            current_timestamp_string()
        ));
        let extract_root = tmp_dir.join("contents");
        fs::create_dir_all(&extract_root).map_err(|error| {
            format!(
                "failed to create temporary extraction dir {}: {error}",
                extract_root.display()
            )
        })?;

        match release.archive_kind {
            ArchiveKind::TarGz => {
                let decoder = GzDecoder::new(Cursor::new(bytes));
                let mut archive = Archive::new(decoder);
                archive.unpack(&extract_root).map_err(|error| {
                    format!("failed to unpack JavaLens tar.gz archive: {error}")
                })?;
            }
            ArchiveKind::Zip => {
                let mut archive = ZipArchive::new(Cursor::new(bytes))
                    .map_err(|error| format!("failed to read JavaLens zip archive: {error}"))?;
                for index in 0..archive.len() {
                    let mut file = archive.by_index(index).map_err(|error| {
                        format!("failed to inspect JavaLens zip entry: {error}")
                    })?;
                    let enclosed = file
                        .enclosed_name()
                        .ok_or("zip archive contained an invalid entry path")?;
                    let output_path = extract_root.join(enclosed);

                    if file.is_dir() {
                        fs::create_dir_all(&output_path).map_err(|error| {
                            format!(
                                "failed to create zip output dir {}: {error}",
                                output_path.display()
                            )
                        })?;
                    } else {
                        if let Some(parent) = output_path.parent() {
                            fs::create_dir_all(parent).map_err(|error| {
                                format!(
                                    "failed to create zip parent dir {}: {error}",
                                    parent.display()
                                )
                            })?;
                        }
                        let mut output_file = fs::File::create(&output_path).map_err(|error| {
                            format!(
                                "failed to create extracted JavaLens file {}: {error}",
                                output_path.display()
                            )
                        })?;
                        std::io::copy(&mut file, &mut output_file).map_err(|error| {
                            format!(
                                "failed to write extracted JavaLens file {}: {error}",
                                output_path.display()
                            )
                        })?;
                    }
                }
            }
        }

        let jar_relative_path = find_relative_jar_path(&extract_root)?;
        if target_dir.exists() {
            fs::remove_dir_all(&target_dir).map_err(|error| {
                format!(
                    "failed to replace managed runtime dir {}: {error}",
                    target_dir.display()
                )
            })?;
        }
        fs::rename(&extract_root, &target_dir).map_err(|error| {
            format!(
                "failed to finalize managed runtime dir {}: {error}",
                target_dir.display()
            )
        })?;
        let _ = fs::remove_dir_all(&tmp_dir);

        let runtime = ManagedRuntimeRecord {
            version: release.version.clone(),
            install_dir: display_path(&target_dir),
            jar_path: display_path(&target_dir.join(&jar_relative_path)),
            asset_name: release.asset_name.clone(),
            installed_at: release
                .published_at
                .clone()
                .unwrap_or_else(current_timestamp_string),
        };

        let manifest = serde_json::to_string_pretty(&runtime)
            .map_err(|error| format!("failed to serialize managed runtime manifest: {error}"))?;
        fs::write(&manifest_path, format!("{manifest}\n")).map_err(|error| {
            format!(
                "failed to write managed runtime manifest {}: {error}",
                manifest_path.display()
            )
        })?;

        Ok(runtime)
    }

    fn build_release_status(
        &self,
        release: Option<&RemoteRelease>,
        installed: &[ManagedRuntimeRecord],
        settings: &ManagerSettings,
        detail_override: Option<String>,
        error_detail: Option<String>,
    ) -> ReleaseStatus {
        if let Some(error_detail) = error_detail {
            return ReleaseStatus {
                kind: ReleaseStatusKind::CheckFailed,
                latest_version: settings.last_seen_latest_version.clone(),
                default_version: settings.default_managed_runtime_version.clone(),
                checked_at: settings.last_release_check.clone(),
                update_available: false,
                detail: error_detail,
            };
        }

        if let Some(release) = release {
            let latest_installed = installed
                .iter()
                .any(|runtime| runtime.version == release.version);
            let update_available = !latest_installed;
            let kind = if installed.is_empty() {
                ReleaseStatusKind::Missing
            } else if update_available {
                ReleaseStatusKind::UpdateAvailable
            } else {
                ReleaseStatusKind::Ready
            };

            return ReleaseStatus {
                kind,
                latest_version: Some(release.version.clone()),
                default_version: settings.default_managed_runtime_version.clone(),
                checked_at: settings.last_release_check.clone(),
                update_available,
                detail: detail_override.unwrap_or_else(|| {
                    if update_available {
                        format!(
                            "Latest upstream release is {}. Download it to keep the managed runtime current.",
                            release.version
                        )
                    } else {
                        format!("Managed JavaLens runtime {} is up to date.", release.version)
                    }
                }),
            };
        }

        ReleaseStatus {
            kind: if installed.is_empty() {
                ReleaseStatusKind::Missing
            } else {
                ReleaseStatusKind::CheckingDisabled
            },
            latest_version: settings.last_seen_latest_version.clone(),
            default_version: settings.default_managed_runtime_version.clone(),
            checked_at: settings.last_release_check.clone(),
            update_available: false,
            detail: detail_override
                .unwrap_or_else(|| "No release information is available yet.".into()),
        }
    }
}

fn normalize_version(tag: &str) -> String {
    tag.trim_start_matches('v').to_string()
}

fn compare_runtime_versions_desc(
    left: &ManagedRuntimeRecord,
    right: &ManagedRuntimeRecord,
) -> std::cmp::Ordering {
    compare_version_strings(&right.version, &left.version)
}

pub fn compare_version_strings(left: &str, right: &str) -> std::cmp::Ordering {
    match (Version::parse(left), Version::parse(right)) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => left.cmp(right),
    }
}

fn find_relative_jar_path(root: &Path) -> Result<PathBuf, String> {
    for entry in WalkDir::new(root) {
        let entry =
            entry.map_err(|error| format!("failed to walk extracted JavaLens archive: {error}"))?;
        if entry.file_type().is_file() && entry.file_name() == "javalens.jar" {
            return entry
                .path()
                .strip_prefix(root)
                .map(PathBuf::from)
                .map_err(|error| {
                    format!("failed to compute extracted JavaLens jar path: {error}")
                });
        }
    }

    Err("downloaded JavaLens archive did not contain javalens.jar".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppPaths;
    use std::path::PathBuf;

    #[test]
    fn compare_version_strings_prefers_newer_semver_tags() {
        assert!(compare_version_strings("1.2.0", "1.1.5").is_gt());
        assert!(compare_version_strings("1.1.5", "1.2.0").is_lt());
        assert!(compare_version_strings("1.2.0", "1.2.0").is_eq());
    }

    #[test]
    fn release_status_marks_update_when_latest_not_installed() {
        let paths = AppPaths {
            config_dir: PathBuf::from("/tmp/config"),
            state_dir: PathBuf::from("/tmp/state"),
            cache_dir: PathBuf::from("/tmp/cache"),
            projects_file: PathBuf::from("/tmp/config/projects.json"),
            settings_file: PathBuf::from("/tmp/config/settings.json"),
            runtime_state_file: PathBuf::from("/tmp/state/runtime-state.json"),
            workspace_root: PathBuf::from("/tmp/cache/workspaces"),
            log_dir: PathBuf::from("/tmp/state/logs"),
            tools_dir: PathBuf::from("/tmp/cache/tools/javalens"),
        };
        let manager = ReleaseManager::new().expect("failed to build release manager");
        let settings = ManagerSettings::default_for_paths(&paths);
        let installed = vec![ManagedRuntimeRecord {
            version: "1.1.5".into(),
            install_dir: "/tmp/cache/tools/javalens/javalens-1.1.5".into(),
            jar_path: "/tmp/cache/tools/javalens/javalens-1.1.5/javalens.jar".into(),
            asset_name: "javalens-v1.1.5.tar.gz".into(),
            installed_at: "123".into(),
        }];
        let release = RemoteRelease {
            version: "1.2.0".into(),
            asset_name: "javalens-v1.2.0.tar.gz".into(),
            download_url: "https://example.com".into(),
            published_at: Some("124".into()),
            archive_kind: ArchiveKind::TarGz,
        };

        let status =
            manager.build_release_status(Some(&release), &installed, &settings, None, None);
        assert!(matches!(status.kind, ReleaseStatusKind::UpdateAvailable));
        assert!(status.update_available);
        assert_eq!(status.latest_version.as_deref(), Some("1.2.0"));
    }
}
