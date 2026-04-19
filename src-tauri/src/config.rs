use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

const APP_NAME: &str = "javalens-manager";
const CONFIG_FILE_NAME: &str = "projects.json";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapStatus {
    pub config_dir: String,
    pub state_dir: String,
    pub cache_dir: String,
    pub config_file: String,
    pub workspace_root: String,
    pub log_dir: String,
    pub transport: String,
    pub health_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRecord {
    pub id: String,
    pub name: String,
    pub project_path: String,
    pub javalens_jar_path: String,
    pub workspace_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddProjectInput {
    pub name: String,
    pub project_path: String,
    pub javalens_jar_path: String,
    pub workspace_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppConfig {
    version: u32,
    projects: Vec<ProjectRecord>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            projects: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub state_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub config_file: PathBuf,
    pub workspace_root: PathBuf,
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
            config_file: config_dir.join(CONFIG_FILE_NAME),
            workspace_root: cache_dir.join("workspaces"),
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
            &self.workspace_root,
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
            config_file: display_path(&self.config_file),
            workspace_root: display_path(&self.workspace_root),
            log_dir: display_path(&self.log_dir),
            transport: "stdio".into(),
            health_strategy: "process-liveness-first".into(),
        }
    }
}

pub struct ConfigStore {
    paths: AppPaths,
    config: Mutex<AppConfig>,
}

impl ConfigStore {
    pub fn new() -> Result<Self, String> {
        let paths = AppPaths::detect()?;
        paths.ensure_dirs()?;

        let config = if paths.config_file.exists() {
            read_config(&paths.config_file)?
        } else {
            let default = AppConfig::default();
            write_config(&paths.config_file, &default)?;
            default
        };

        Ok(Self {
            paths,
            config: Mutex::new(config),
        })
    }

    pub fn paths(&self) -> AppPaths {
        self.paths.clone()
    }

    pub fn bootstrap_status(&self) -> BootstrapStatus {
        self.paths.bootstrap_status()
    }

    pub fn list_projects(&self) -> Vec<ProjectRecord> {
        self.config
            .lock()
            .expect("config mutex poisoned")
            .projects
            .clone()
    }

    pub fn get_project(&self, project_id: &str) -> Option<ProjectRecord> {
        self.config
            .lock()
            .expect("config mutex poisoned")
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .cloned()
    }

    pub fn add_project(&self, input: AddProjectInput) -> Result<ProjectRecord, String> {
        validate_non_empty("name", &input.name)?;
        validate_non_empty("projectPath", &input.project_path)?;
        validate_non_empty("javalensJarPath", &input.javalens_jar_path)?;

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
            javalens_jar_path: input.javalens_jar_path.trim().to_string(),
            workspace_dir,
        };

        let mut config = self.config.lock().expect("config mutex poisoned");

        if config
            .projects
            .iter()
            .any(|existing| existing.project_path == project.project_path)
        {
            return Err("A project with the same project path is already registered".into());
        }

        config.projects.push(project.clone());
        write_config(&self.paths.config_file, &config)?;

        Ok(project)
    }
}

fn validate_non_empty(field_name: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field_name} must not be empty"));
    }

    Ok(())
}

fn read_config(path: &Path) -> Result<AppConfig, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

    serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn write_config(path: &Path, config: &AppConfig) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config)
        .map_err(|error| format!("failed to serialize config: {error}"))?;

    fs::write(path, format!("{json}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn current_timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis()
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
            config_file: PathBuf::from("/tmp/config/projects.json"),
            workspace_root: PathBuf::from("/tmp/cache/workspaces"),
            log_dir: PathBuf::from("/tmp/state/logs"),
        };

        let bootstrap = paths.bootstrap_status();
        assert_eq!(bootstrap.transport, "stdio");
        assert_eq!(bootstrap.health_strategy, "process-liveness-first");
    }
}
