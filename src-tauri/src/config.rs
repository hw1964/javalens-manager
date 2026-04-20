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
    pub default_data_root: String,
    pub log_dir: String,
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
            last_release_check: None,
            last_seen_latest_version: None,
        }
    }

    pub fn tools_dir(&self) -> PathBuf {
        PathBuf::from(&self.data_root).join("tools").join("javalens")
    }

    pub fn workspace_root(&self) -> PathBuf {
        PathBuf::from(&self.data_root).join("workspaces")
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRecord {
    pub id: String,
    pub name: String,
    pub project_path: String,
    pub assigned_port: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddProjectInput {
    pub name: String,
    pub project_path: String,
    pub assigned_port: Option<u16>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsInput {
    pub update_policy: UpdatePolicy,
    pub auto_check_for_updates: bool,
    pub data_root: String,
    pub global_runtime_source: RuntimeSource,
    pub port_range_start: u16,
    pub port_range_end: u16,
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

    pub fn update_project_port(&self, project_id: &str, assigned_port: u16) -> Result<ProjectRecord, String> {
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
