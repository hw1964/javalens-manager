use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

const APP_NAME: &str = "javalens-manager";
const PROJECTS_FILE_NAME: &str = "projects.json";
const SETTINGS_FILE_NAME: &str = "settings.json";
const RUNTIME_STATE_FILE_NAME: &str = "runtime-state.json";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapStatus {
    pub config_dir: String,
    pub state_dir: String,
    pub cache_dir: String,
    pub projects_file: String,
    pub settings_file: String,
    pub runtime_state_file: String,
    pub workspace_root: String,
    pub log_dir: String,
    pub tools_dir: String,
    pub transport: String,
    pub health_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum UpdatePolicy {
    Always,
    Ask,
}

impl Default for UpdatePolicy {
    fn default() -> Self {
        Self::Ask
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagerSettings {
    pub version: u32,
    pub update_policy: UpdatePolicy,
    pub auto_check_for_updates: bool,
    pub default_managed_runtime_version: Option<String>,
    pub manual_fallback_jar_path: Option<String>,
    pub tools_dir: String,
    pub last_release_check: Option<String>,
    pub last_seen_latest_version: Option<String>,
}

impl ManagerSettings {
    pub(crate) fn default_for_paths(paths: &AppPaths) -> Self {
        Self {
            version: 1,
            update_policy: UpdatePolicy::Ask,
            auto_check_for_updates: true,
            default_managed_runtime_version: None,
            manual_fallback_jar_path: None,
            tools_dir: display_path(&paths.tools_dir),
            last_release_check: None,
            last_seen_latest_version: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum RuntimeSource {
    Managed { version: String },
    LocalJar { jar_path: String },
}

impl RuntimeSource {
    pub fn label(&self) -> String {
        match self {
            RuntimeSource::Managed { version } => format!("Managed JavaLens {version}"),
            RuntimeSource::LocalJar { jar_path } => format!("Local JAR ({jar_path})"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRecord {
    pub id: String,
    pub name: String,
    pub project_path: String,
    pub runtime_source: RuntimeSource,
    pub workspace_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddProjectInput {
    pub name: String,
    pub project_path: String,
    pub runtime_source: RuntimeSource,
    pub workspace_dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsInput {
    pub update_policy: UpdatePolicy,
    pub auto_check_for_updates: bool,
    pub default_managed_runtime_version: Option<String>,
    pub tools_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ProjectsFile {
    version: u32,
    projects: Vec<ProjectRecord>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyProjectRecord {
    id: String,
    name: String,
    project_path: String,
    javalens_jar_path: String,
    workspace_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyProjectsFile {
    version: Option<u32>,
    projects: Vec<LegacyProjectRecord>,
}

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub state_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub projects_file: PathBuf,
    pub settings_file: PathBuf,
    pub runtime_state_file: PathBuf,
    pub workspace_root: PathBuf,
    pub log_dir: PathBuf,
    pub tools_dir: PathBuf,
}

impl AppPaths {
    pub fn detect() -> Result<Self, String> {
        let home_dir = dirs::home_dir().ok_or("Could not determine user home directory")?;
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| home_dir.join(".config"))
            .join(APP_NAME);
        let state_dir = dirs::state_dir()
            .or_else(dirs::data_local_dir)
            .unwrap_or_else(|| home_dir.join(".local").join("state"))
            .join(APP_NAME);
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| home_dir.join(".cache"))
            .join(APP_NAME);

        Ok(Self {
            projects_file: config_dir.join(PROJECTS_FILE_NAME),
            settings_file: config_dir.join(SETTINGS_FILE_NAME),
            runtime_state_file: state_dir.join(RUNTIME_STATE_FILE_NAME),
            workspace_root: cache_dir.join("workspaces"),
            log_dir: state_dir.join("logs"),
            tools_dir: cache_dir.join("tools").join("javalens"),
            config_dir,
            state_dir,
            cache_dir,
        })
    }

    pub fn ensure_dirs(&self) -> Result<(), String> {
        for dir in [
            &self.config_dir,
            &self.state_dir,
            &self.cache_dir,
            &self.workspace_root,
            &self.log_dir,
            &self.tools_dir,
        ] {
            fs::create_dir_all(dir)
                .map_err(|error| format!("failed to create {}: {error}", dir.display()))?;
        }

        Ok(())
    }

    pub fn bootstrap_status(&self) -> BootstrapStatus {
        BootstrapStatus {
            config_dir: display_path(&self.config_dir),
            state_dir: display_path(&self.state_dir),
            cache_dir: display_path(&self.cache_dir),
            projects_file: display_path(&self.projects_file),
            settings_file: display_path(&self.settings_file),
            runtime_state_file: display_path(&self.runtime_state_file),
            workspace_root: display_path(&self.workspace_root),
            log_dir: display_path(&self.log_dir),
            tools_dir: display_path(&self.tools_dir),
            transport: "stdio".into(),
            health_strategy: "process-liveness-first".into(),
        }
    }
}

pub struct ConfigStore {
    paths: AppPaths,
    projects: Mutex<ProjectsFile>,
    settings: Mutex<ManagerSettings>,
}

impl ConfigStore {
    pub fn new() -> Result<Self, String> {
        let paths = AppPaths::detect()?;
        paths.ensure_dirs()?;

        let projects = if paths.projects_file.exists() {
            read_projects(&paths.projects_file)?
        } else {
            let default = ProjectsFile {
                version: 1,
                projects: Vec::new(),
            };
            write_json(&paths.projects_file, &default)?;
            default
        };

        let settings = if paths.settings_file.exists() {
            read_settings(&paths.settings_file, &paths)?
        } else {
            let default = ManagerSettings::default_for_paths(&paths);
            write_json(&paths.settings_file, &default)?;
            default
        };

        Ok(Self {
            paths,
            projects: Mutex::new(projects),
            settings: Mutex::new(settings),
        })
    }

    pub fn paths(&self) -> AppPaths {
        self.paths.clone()
    }

    pub fn bootstrap_status(&self) -> BootstrapStatus {
        self.paths.bootstrap_status()
    }

    pub fn list_projects(&self) -> Vec<ProjectRecord> {
        self.projects
            .lock()
            .expect("projects mutex poisoned")
            .projects
            .clone()
    }

    pub fn get_project(&self, project_id: &str) -> Option<ProjectRecord> {
        self.projects
            .lock()
            .expect("projects mutex poisoned")
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .cloned()
    }

    pub fn add_project(&self, input: AddProjectInput) -> Result<ProjectRecord, String> {
        validate_non_empty("name", &input.name)?;
        validate_non_empty("projectPath", &input.project_path)?;
        validate_runtime_source(&input.runtime_source)?;

        let project_slug = slugify(&input.name);
        let project_id = format!("{project_slug}-{}", current_timestamp_millis());
        let workspace_dir = input
            .workspace_dir
            .filter(|path| !path.trim().is_empty())
            .unwrap_or_else(|| display_path(&self.paths.workspace_root.join(&project_id)));

        let project = ProjectRecord {
            id: project_id,
            name: input.name.trim().to_string(),
            project_path: input.project_path.trim().to_string(),
            runtime_source: input.runtime_source,
            workspace_dir,
        };

        let mut projects = self.projects.lock().expect("projects mutex poisoned");

        if projects
            .projects
            .iter()
            .any(|existing| existing.project_path == project.project_path)
        {
            return Err("A project with the same project path is already registered".into());
        }

        projects.projects.push(project.clone());
        write_json(&self.paths.projects_file, &*projects)?;

        Ok(project)
    }

    pub fn get_settings(&self) -> ManagerSettings {
        self.settings
            .lock()
            .expect("settings mutex poisoned")
            .clone()
    }

    pub fn update_settings(&self, input: UpdateSettingsInput) -> Result<ManagerSettings, String> {
        let mut settings = self.settings.lock().expect("settings mutex poisoned");
        settings.update_policy = input.update_policy;
        settings.auto_check_for_updates = input.auto_check_for_updates;
        settings.default_managed_runtime_version = input.default_managed_runtime_version;
        
        if input.tools_dir.trim().is_empty() {
            return Err("toolsDir must not be empty".into());
        }
        settings.tools_dir = input.tools_dir.trim().to_string();
        
        write_json(&self.paths.settings_file, &*settings)?;
        Ok(settings.clone())
    }

    pub fn write_settings(&self, settings: ManagerSettings) -> Result<ManagerSettings, String> {
        let mut guard = self.settings.lock().expect("settings mutex poisoned");
        *guard = settings.clone();
        write_json(&self.paths.settings_file, &settings)?;
        Ok(settings)
    }
}

fn validate_non_empty(field_name: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field_name} must not be empty"));
    }

    Ok(())
}

fn validate_runtime_source(runtime_source: &RuntimeSource) -> Result<(), String> {
    match runtime_source {
        RuntimeSource::Managed { version } => validate_non_empty("runtimeSource.version", version),
        RuntimeSource::LocalJar { jar_path } => {
            validate_non_empty("runtimeSource.jarPath", jar_path)
        }
    }
}

fn read_projects(path: &Path) -> Result<ProjectsFile, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

    if let Ok(projects) = serde_json::from_str::<ProjectsFile>(&contents) {
        return Ok(projects);
    }

    let legacy = serde_json::from_str::<LegacyProjectsFile>(&contents)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    Ok(ProjectsFile {
        version: legacy.version.unwrap_or(1),
        projects: legacy
            .projects
            .into_iter()
            .map(|legacy_project| ProjectRecord {
                id: legacy_project.id,
                name: legacy_project.name,
                project_path: legacy_project.project_path,
                runtime_source: RuntimeSource::LocalJar {
                    jar_path: legacy_project.javalens_jar_path,
                },
                workspace_dir: legacy_project.workspace_dir,
            })
            .collect(),
    })
}

fn read_settings(path: &Path, paths: &AppPaths) -> Result<ManagerSettings, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

    let mut settings: ManagerSettings = serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    if settings.tools_dir.trim().is_empty() {
        settings.tools_dir = display_path(&paths.tools_dir);
    }
    Ok(settings)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize {}: {error}", path.display()))?;
    fs::write(path, format!("{json}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn current_timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis()
}

pub fn current_timestamp_string() -> String {
    current_timestamp_millis().to_string()
}

pub fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }

    let slug = slug.trim_matches('-');

    if slug.is_empty() {
        "project".into()
    } else {
        slug.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_normalizes_display_names() {
        assert_eq!(slugify("Example Service"), "example-service");
        assert_eq!(slugify("Repo::Manager"), "repo-manager");
        assert_eq!(slugify("###"), "project");
    }

    #[test]
    fn bootstrap_status_uses_stdio_transport() {
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

        let bootstrap = paths.bootstrap_status();
        assert_eq!(bootstrap.transport, "stdio");
        assert_eq!(bootstrap.health_strategy, "process-liveness-first");
        assert!(bootstrap.settings_file.ends_with("settings.json"));
        assert!(bootstrap.tools_dir.ends_with("tools/javalens"));
    }

    #[test]
    fn legacy_project_shape_is_upgraded_to_local_runtime_source() {
        let legacy = r#"{
          "version": 1,
          "projects": [
            {
              "id": "legacy-1",
              "name": "Legacy",
              "projectPath": "/tmp/project",
              "javalensJarPath": "/tmp/javalens.jar",
              "workspaceDir": "/tmp/workspace"
            }
          ]
        }"#;

        let path = PathBuf::from("/tmp/legacy-projects.json");
        fs::write(&path, legacy).expect("failed to write test file");
        let parsed = read_projects(&path).expect("failed to parse legacy projects");
        let _ = fs::remove_file(&path);

        assert_eq!(parsed.projects.len(), 1);
        match &parsed.projects[0].runtime_source {
            RuntimeSource::LocalJar { jar_path } => assert_eq!(jar_path, "/tmp/javalens.jar"),
            RuntimeSource::Managed { .. } => panic!("legacy shape should map to LocalJar"),
        }
    }

    #[test]
    fn settings_defaults_use_ask_policy_and_auto_checks() {
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

        let settings = ManagerSettings::default_for_paths(&paths);
        assert_eq!(settings.update_policy, UpdatePolicy::Ask);
        assert!(settings.auto_check_for_updates);
        assert_eq!(settings.tools_dir, "/tmp/cache/tools/javalens");
    }
}
