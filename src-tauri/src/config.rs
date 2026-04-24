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

/// Initial configuration and state paths required for the application to start.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapStatus {
    pub config_dir: String,
    pub state_dir: String,
    pub cache_dir: String,
    pub projects_file: String,
    pub settings_file: String,
    pub runtime_state_file: String,
    pub default_data_root: String,
    pub log_dir: String,
    pub transport: String,
    pub health_strategy: String,
}

/// Policy determining how application updates should be handled.
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

fn default_data_root() -> String {
    String::new()
}

fn default_global_runtime_source() -> RuntimeSource {
    RuntimeSource::Managed
}

fn default_port_range_start() -> u16 {
    11100
}

fn default_port_range_end() -> u16 {
    11199
}

fn default_use_system_tray() -> bool {
    true
}

fn default_mcp_merge_mode() -> McpMergeMode {
    McpMergeMode::SafeMerge
}

fn default_mcp_backup_before_write() -> bool {
    true
}

fn default_deploy_targets() -> DeployTargetFlags {
    DeployTargetFlags::default()
}

/// Strategy for merging MCP configuration changes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum McpMergeMode {
    SafeMerge,
    ReplaceManagedSection,
}

impl Default for McpMergeMode {
    fn default() -> Self {
        McpMergeMode::SafeMerge
    }
}

/// Configuration for a specific MCP client's configuration file path.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct McpClientPathEntry {
    #[serde(default)]
    pub auto_detected_path: Option<String>,
    #[serde(default)]
    pub manual_override_path: Option<String>,
    #[serde(default)]
    pub effective_path: Option<String>,
}

/// Collection of paths to various MCP client configuration files.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct McpClientPaths {
    #[serde(default)]
    pub cursor: McpClientPathEntry,
    #[serde(default)]
    pub claude: McpClientPathEntry,
    #[serde(default)]
    pub antigravity: McpClientPathEntry,
    #[serde(default)]
    pub intellij: McpClientPathEntry,
}

/// Flags indicating which MCP clients should receive deployments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployTargetFlags {
    #[serde(default = "default_enabled_flag")]
    pub cursor: bool,
    #[serde(default = "default_enabled_flag")]
    pub claude: bool,
    #[serde(default = "default_enabled_flag")]
    pub antigravity: bool,
    #[serde(default = "default_enabled_flag")]
    pub intellij: bool,
}

fn default_enabled_flag() -> bool {
    true
}

impl Default for DeployTargetFlags {
    fn default() -> Self {
        Self {
            cursor: true,
            claude: true,
            antigravity: true,
            intellij: true,
        }
    }
}

fn default_mcp_client_paths() -> McpClientPaths {
    detect_default_mcp_client_paths()
}

/// Global settings for the JavaLens manager application.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagerSettings {
    pub version: u32,
    pub update_policy: UpdatePolicy,
    pub auto_check_for_updates: bool,
    pub manual_fallback_jar_path: Option<String>,
    #[serde(default = "default_data_root")]
    pub data_root: String,
    #[serde(default = "default_global_runtime_source")]
    pub global_runtime_source: RuntimeSource,
    #[serde(default = "default_port_range_start")]
    pub port_range_start: u16,
    #[serde(default = "default_port_range_end")]
    pub port_range_end: u16,
    #[serde(default = "default_use_system_tray")]
    pub use_system_tray: bool,
    #[serde(default = "default_mcp_client_paths")]
    pub mcp_client_paths: McpClientPaths,
    #[serde(default = "default_mcp_merge_mode")]
    pub mcp_merge_mode: McpMergeMode,
    #[serde(default = "default_mcp_backup_before_write")]
    pub mcp_backup_before_write: bool,
    #[serde(default = "default_deploy_targets")]
    pub deploy_targets: DeployTargetFlags,
    pub last_release_check: Option<String>,
    pub last_seen_latest_version: Option<String>,
}

impl ManagerSettings {
    pub(crate) fn default_for_paths(paths: &AppPaths) -> Self {
        Self {
            version: 1,
            update_policy: UpdatePolicy::Ask,
            auto_check_for_updates: true,
            manual_fallback_jar_path: None,
            data_root: display_path(&paths.default_data_root),
            global_runtime_source: RuntimeSource::Managed,
            port_range_start: default_port_range_start(),
            port_range_end: default_port_range_end(),
            use_system_tray: default_use_system_tray(),
            mcp_client_paths: detect_default_mcp_client_paths(),
            mcp_merge_mode: default_mcp_merge_mode(),
            mcp_backup_before_write: default_mcp_backup_before_write(),
            deploy_targets: default_deploy_targets(),
            last_release_check: None,
            last_seen_latest_version: None,
        }
    }

    pub fn tools_dir(&self) -> PathBuf {
        PathBuf::from(&self.data_root)
            .join("tools")
            .join("javalens")
    }

    pub fn workspace_root(&self) -> PathBuf {
        PathBuf::from(&self.data_root).join("workspaces")
    }
}

/// Source of the JavaLens runtime environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum RuntimeSource {
    Managed,
    LocalJar { jar_path: String },
}

impl RuntimeSource {
    pub fn label(&self) -> String {
        match self {
            RuntimeSource::Managed => "Managed JavaLens (Latest)".into(),
            RuntimeSource::LocalJar { jar_path } => format!("Local JAR ({jar_path})"),
        }
    }
}

/// Information about a registered Java project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRecord {
    pub id: String,
    pub name: String,
    pub project_path: String,
    pub assigned_port: u16,
}

/// Input data for registering a new project.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddProjectInput {
    pub name: String,
    pub project_path: String,
    pub assigned_port: Option<u16>,
}

/// Input data for updating the manager settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsInput {
    pub update_policy: UpdatePolicy,
    pub auto_check_for_updates: bool,
    pub data_root: String,
    pub global_runtime_source: RuntimeSource,
    pub port_range_start: u16,
    pub port_range_end: u16,
    pub use_system_tray: bool,
    pub mcp_client_paths: McpClientPaths,
    pub mcp_merge_mode: McpMergeMode,
    pub mcp_backup_before_write: bool,
    pub deploy_targets: DeployTargetFlags,
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

/// Core filesystem paths used by the application.
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub state_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub projects_file: PathBuf,
    pub settings_file: PathBuf,
    pub runtime_state_file: PathBuf,
    pub default_data_root: PathBuf,
    pub log_dir: PathBuf,
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
            default_data_root: cache_dir.clone(),
            log_dir: state_dir.join("logs"),
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
            &self.default_data_root,
            &self.log_dir,
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
            default_data_root: display_path(&self.default_data_root),
            log_dir: display_path(&self.log_dir),
            transport: "stdio".into(),
            health_strategy: "process-liveness-first".into(),
        }
    }
}

/// Thread-safe storage for application configuration and state.
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
        let assigned_port = input
            .assigned_port
            .ok_or("assignedPort must be set before adding project")?;

        let project_slug = slugify(&input.name);
        let project_id = format!("{project_slug}-{}", current_timestamp_millis());

        let project = ProjectRecord {
            id: project_id,
            name: input.name.trim().to_string(),
            project_path: input.project_path.trim().to_string(),
            assigned_port,
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

    pub fn update_project_port(
        &self,
        project_id: &str,
        assigned_port: u16,
    ) -> Result<ProjectRecord, String> {
        let mut projects = self.projects.lock().expect("projects mutex poisoned");
        let project = projects
            .projects
            .iter_mut()
            .find(|project| project.id == project_id)
            .ok_or_else(|| format!("Unknown project id: {project_id}"))?;
        project.assigned_port = assigned_port;
        let updated = project.clone();
        write_json(&self.paths.projects_file, &*projects)?;
        Ok(updated)
    }

    pub fn delete_project(&self, project_id: &str) -> Result<ProjectRecord, String> {
        let mut projects = self.projects.lock().expect("projects mutex poisoned");
        let index = projects
            .projects
            .iter()
            .position(|project| project.id == project_id)
            .ok_or_else(|| format!("Unknown project id: {project_id}"))?;
        let removed = projects.projects.remove(index);
        write_json(&self.paths.projects_file, &*projects)?;
        Ok(removed)
    }

    pub fn used_ports(&self, excluding_project_id: Option<&str>) -> Vec<u16> {
        self.projects
            .lock()
            .expect("projects mutex poisoned")
            .projects
            .iter()
            .filter(|project| excluding_project_id.map_or(true, |id| project.id != id))
            .map(|project| project.assigned_port)
            .collect()
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

        if input.data_root.trim().is_empty() {
            return Err("dataRoot must not be empty".into());
        }
        settings.data_root = input.data_root.trim().to_string();

        validate_runtime_source(&input.global_runtime_source)?;
        settings.global_runtime_source = input.global_runtime_source;
        validate_port_range(input.port_range_start, input.port_range_end)?;
        settings.port_range_start = input.port_range_start;
        settings.port_range_end = input.port_range_end;
        settings.use_system_tray = input.use_system_tray;
        settings.mcp_client_paths = sanitize_mcp_client_paths(input.mcp_client_paths);
        settings.mcp_merge_mode = input.mcp_merge_mode;
        settings.mcp_backup_before_write = input.mcp_backup_before_write;
        settings.deploy_targets = sanitize_deploy_target_flags(input.deploy_targets);

        write_json(&self.paths.settings_file, &*settings)?;
        Ok(settings.clone())
    }

    pub fn redetect_mcp_client_paths(&self) -> Result<ManagerSettings, String> {
        let mut settings = self.settings.lock().expect("settings mutex poisoned");
        settings.mcp_client_paths = merge_detected_mcp_paths(settings.mcp_client_paths.clone());
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
        RuntimeSource::Managed => Ok(()),
        RuntimeSource::LocalJar { jar_path } => {
            validate_non_empty("runtimeSource.jarPath", jar_path)
        }
    }
}

fn validate_port_range(start: u16, end: u16) -> Result<(), String> {
    if start > end {
        return Err("portRangeStart must be <= portRangeEnd".into());
    }
    if start < 1024 {
        return Err("portRangeStart must be >= 1024".into());
    }
    Ok(())
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
                assigned_port: default_port_range_start(),
            })
            .collect(),
    })
}

fn read_settings(path: &Path, paths: &AppPaths) -> Result<ManagerSettings, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

    let mut settings: ManagerSettings = serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    if settings.data_root.trim().is_empty() {
        settings.data_root = display_path(&paths.default_data_root);
    }
    if validate_port_range(settings.port_range_start, settings.port_range_end).is_err() {
        settings.port_range_start = default_port_range_start();
        settings.port_range_end = default_port_range_end();
    }
    settings.mcp_client_paths = merge_detected_mcp_paths(settings.mcp_client_paths);
    Ok(settings)
}

fn detect_default_mcp_client_paths() -> McpClientPaths {
    let home = dirs::home_dir();
    let detect = |candidates: &[PathBuf]| -> Option<String> {
        candidates
            .iter()
            .find(|path| path.exists())
            .map(|path| display_path(path))
            .or_else(|| candidates.first().map(|path| display_path(path)))
    };

    let build = |parts: &[&str]| -> Option<PathBuf> {
        home.as_ref()
            .map(|h| parts.iter().fold(h.clone(), |acc, part| acc.join(part)))
    };

    let cursor_candidates: Vec<PathBuf> = [
        [".cursor", "mcp.json"].as_slice(),
        [".config", "Cursor", "mcp.json"].as_slice(),
    ]
    .iter()
    .filter_map(|parts| build(parts))
    .collect();

    let claude_candidates: Vec<PathBuf> = [
        [".claude", "mcp.json"].as_slice(),
        [".claude.json"].as_slice(),
    ]
    .iter()
    .filter_map(|parts| build(parts))
    .collect();

    let antigravity_candidates: Vec<PathBuf> = [
        [".config", "Antigravity", "User", "mcp.json"].as_slice(),
        [".antigravity", "mcp.json"].as_slice(),
        [".config", "antigravity", "mcp.json"].as_slice(),
    ]
    .iter()
    .filter_map(|parts| build(parts))
    .collect();

    let intellij_candidates: Vec<PathBuf> = [
        [".config", "JetBrains", "IntelliJIdea", "mcp.json"].as_slice(),
        [".IntelliJIdea", "config", "options", "mcp.json"].as_slice(),
    ]
    .iter()
    .filter_map(|parts| build(parts))
    .collect();

    let make_entry = |candidates: &[PathBuf]| McpClientPathEntry {
        auto_detected_path: detect(candidates),
        manual_override_path: None,
        effective_path: detect(candidates),
    };

    McpClientPaths {
        cursor: make_entry(&cursor_candidates),
        claude: make_entry(&claude_candidates),
        antigravity: make_entry(&antigravity_candidates),
        intellij: make_entry(&intellij_candidates),
    }
}

fn merge_detected_mcp_paths(paths: McpClientPaths) -> McpClientPaths {
    let defaults = detect_default_mcp_client_paths();
    McpClientPaths {
        cursor: merge_mcp_path_entry(paths.cursor, defaults.cursor),
        claude: merge_mcp_path_entry(paths.claude, defaults.claude),
        antigravity: merge_mcp_path_entry(paths.antigravity, defaults.antigravity),
        intellij: merge_mcp_path_entry(paths.intellij, defaults.intellij),
    }
}

fn merge_mcp_path_entry(
    mut current: McpClientPathEntry,
    detected: McpClientPathEntry,
) -> McpClientPathEntry {
    current.auto_detected_path = detected.auto_detected_path;
    let manual = current
        .manual_override_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    current.manual_override_path = manual.clone();
    current.effective_path = manual.or_else(|| current.auto_detected_path.clone());
    current
}

fn sanitize_mcp_client_paths(paths: McpClientPaths) -> McpClientPaths {
    merge_detected_mcp_paths(paths)
}

fn sanitize_deploy_target_flags(flags: DeployTargetFlags) -> DeployTargetFlags {
    flags
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if path.exists() {
        let backup_path = path.with_extension(format!("json.bak.{}", current_timestamp_millis()));
        if let Err(error) = fs::copy(path, &backup_path) {
            eprintln!(
                "Warning: failed to create backup of {}: {error}",
                path.display()
            );
        }
    }

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

/// Returns the current UNIX timestamp in milliseconds as a string.
pub fn current_timestamp_string() -> String {
    current_timestamp_millis().to_string()
}

/// Converts a path to a string, using lossy conversion if necessary.
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
            default_data_root: PathBuf::from("/tmp/cache/javalens-manager"),
            log_dir: PathBuf::from("/tmp/state/logs"),
        };

        let bootstrap = paths.bootstrap_status();
        assert_eq!(bootstrap.transport, "stdio");
        assert_eq!(bootstrap.health_strategy, "process-liveness-first");
        assert!(bootstrap.settings_file.ends_with("settings.json"));
        assert!(bootstrap.default_data_root.ends_with("javalens-manager"));
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
        assert_eq!(parsed.projects[0].id, "legacy-1");
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
            default_data_root: PathBuf::from("/tmp/cache/javalens-manager"),
            log_dir: PathBuf::from("/tmp/state/logs"),
        };

        let settings = ManagerSettings::default_for_paths(&paths);
        assert_eq!(settings.update_policy, UpdatePolicy::Ask);
        assert!(settings.auto_check_for_updates);
        assert_eq!(settings.data_root, "/tmp/cache/javalens-manager");
    }
}
