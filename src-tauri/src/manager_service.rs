use crate::{
    config::{
        AddProjectInput, BootstrapStatus, ConfigStore, ManagerSettings, ProjectRecord,
        RuntimeSource, UpdateSettingsInput,
    },
    release_manager::{ManagedRuntimeRecord, ReleaseManager, ReleaseStatus},
    runtime_manager::{RuntimeLaunchRequest, RuntimeManager, RuntimeReference, RuntimeStatusRecord},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    net::TcpListener,
    path::{Path, PathBuf},
};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagerDashboard {
    pub bootstrap: BootstrapStatus,
    pub settings: ManagerSettings,
    pub release_status: ReleaseStatus,
    pub installed_runtime: Option<ManagedRuntimeRecord>,
    pub projects: Vec<ProjectRecord>,
    pub runtime_statuses: HashMap<String, RuntimeStatusRecord>,
    pub suggested_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceProjectCandidate {
    pub name: String,
    pub project_path: String,
    pub kind: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectPortInput {
    pub project_id: String,
    pub assigned_port: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceImportInput {
    pub workspace_file: String,
    pub selected_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceImportResult {
    pub added: Vec<ProjectRecord>,
    pub skipped: Vec<String>,
}

pub struct ManagerService {
    config_store: ConfigStore,
    release_manager: ReleaseManager,
    runtime_manager: RuntimeManager,
}

impl ManagerService {
    pub fn new(
        config_store: ConfigStore,
        release_manager: ReleaseManager,
        runtime_manager: RuntimeManager,
    ) -> Self {
        Self {
            config_store,
            release_manager,
            runtime_manager,
        }
    }

    pub fn load_dashboard(&self) -> Result<ManagerDashboard, String> {
        let bootstrap = self.config_store.bootstrap_status();
        let mut settings = self.config_store.get_settings();
        let (installed_runtime, release_status) = self.release_manager.sync_with_settings(&mut settings)?;
        let settings = self.config_store.write_settings(settings)?;

        let projects = self.config_store.list_projects();
        let runtime_statuses =
            self.collect_runtime_statuses(&projects, &settings, installed_runtime.as_ref());
        let suggested_port = self.suggest_next_port_for(&settings, None);

        Ok(ManagerDashboard {
            bootstrap,
            settings,
            release_status,
            installed_runtime,
            projects,
            runtime_statuses,
            suggested_port,
        })
    }

    pub fn suggest_next_port(&self) -> Result<u16, String> {
        let settings = self.config_store.get_settings();
        self.suggest_next_port_for(&settings, None)
            .ok_or("No free port available in configured range".into())
    }

    pub fn add_project(&self, input: AddProjectInput) -> Result<ProjectRecord, String> {
        let settings = self.config_store.get_settings();
        let assigned_port = self.allocate_port(&settings, None, input.assigned_port)?;
        self.config_store.add_project(AddProjectInput {
            name: input.name,
            project_path: input.project_path,
            assigned_port: Some(assigned_port),
        })
    }

    pub fn update_project_port(&self, input: UpdateProjectPortInput) -> Result<ManagerDashboard, String> {
        let settings = self.config_store.get_settings();
        let assigned_port =
            self.allocate_port(&settings, Some(input.project_id.as_str()), Some(input.assigned_port))?;
        self.config_store
            .update_project_port(&input.project_id, assigned_port)?;
        self.load_dashboard()
    }

    pub fn delete_project(&self, project_id: &str) -> Result<ManagerDashboard, String> {
        self.runtime_manager.remove_project_runtime(project_id)?;
        self.config_store.delete_project(project_id)?;
        self.load_dashboard()
    }

    pub fn start_all_runtimes(&self) -> Result<ManagerDashboard, String> {
        let projects = self.config_store.list_projects();
        let mut errors = Vec::new();

        for project in projects {
            match self.resolve_launch_request(&project) {
                Ok(launch_request) => {
                    if let Err(error) = self.runtime_manager.start_runtime(&launch_request) {
                        errors.push(format!("{}: {error}", project.name));
                    }
                }
                Err(error) => errors.push(format!("{}: {error}", project.name)),
            }
        }

        if !errors.is_empty() {
            return Err(format!("Some runtimes failed to start: {}", errors.join(" | ")));
        }

        self.load_dashboard()
    }

    pub fn delete_all_projects(&self) -> Result<ManagerDashboard, String> {
        let project_ids: Vec<String> = self
            .config_store
            .list_projects()
            .into_iter()
            .map(|project| project.id)
            .collect();

        for project_id in project_ids {
            self.runtime_manager.remove_project_runtime(&project_id)?;
            self.config_store.delete_project(&project_id)?;
        }

        self.load_dashboard()
    }

    pub fn update_settings(&self, input: UpdateSettingsInput) -> Result<ManagerDashboard, String> {
        self.config_store.update_settings(input)?;
        self.load_dashboard()
    }

    pub fn download_or_update_javalens(&self) -> Result<ManagerDashboard, String> {
        let mut settings = self.config_store.get_settings();
        self.release_manager
            .download_latest_runtime(&mut settings)?;
        self.config_store.write_settings(settings)?;
        self.load_dashboard()
    }

    pub fn discover_workspace_projects(
        &self,
        workspace_file: &str,
    ) -> Result<Vec<WorkspaceProjectCandidate>, String> {
        let roots = read_workspace_roots(workspace_file)?;
        let mut by_path: HashMap<String, WorkspaceProjectCandidate> = HashMap::new();

        for root in roots {
            if !root.exists() {
                continue;
            }
            for entry in WalkDir::new(&root)
                .follow_links(false)
                .max_depth(6)
                .into_iter()
                .filter_entry(should_walk_entry)
            {
                let entry = entry.map_err(|error| format!("workspace scan failed: {error}"))?;
                if !entry.file_type().is_dir() {
                    continue;
                }
                let path = entry.path();
                if is_ignored_candidate_path(path) {
                    continue;
                }
                if let Some(kind) = detect_java_project_kind(path) {
                    let key = path.to_string_lossy().to_string();
                    by_path.entry(key.clone()).or_insert_with(|| WorkspaceProjectCandidate {
                        name: path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "project".into()),
                        project_path: key,
                        kind,
                    });
                }
            }
        }

        let mut candidates: Vec<_> = by_path.into_values().collect();
        candidates.sort_by(|a, b| {
            let al = a.project_path.len();
            let bl = b.project_path.len();
            al.cmp(&bl).then(a.project_path.cmp(&b.project_path))
        });

        // Keep only containing project roots; drop nested children.
        let mut filtered: Vec<WorkspaceProjectCandidate> = Vec::new();
        for candidate in candidates {
            let candidate_path = PathBuf::from(&candidate.project_path);
            let is_nested = filtered
                .iter()
                .map(|parent| PathBuf::from(&parent.project_path))
                .any(|parent| candidate_path != parent && candidate_path.starts_with(&parent));
            if !is_nested {
                filtered.push(candidate);
            }
        }
        filtered.sort_by(|a, b| a.project_path.cmp(&b.project_path));
        Ok(filtered)
    }

    pub fn import_workspace_projects(
        &self,
        input: WorkspaceImportInput,
    ) -> Result<WorkspaceImportResult, String> {
        let candidates = self.discover_workspace_projects(&input.workspace_file)?;
        let selected: HashSet<String> = input.selected_paths.into_iter().collect();
        let mut added = Vec::new();
        let mut skipped = Vec::new();

        for candidate in candidates {
            if !selected.contains(&candidate.project_path) {
                continue;
            }
            let result = self.add_project(AddProjectInput {
                name: candidate.name.clone(),
                project_path: candidate.project_path.clone(),
                assigned_port: None,
            });
            match result {
                Ok(project) => added.push(project),
                Err(error) => skipped.push(format!("{} ({error})", candidate.project_path)),
            }
        }

        Ok(WorkspaceImportResult { added, skipped })
    }

    pub fn start_runtime(&self, project_id: &str) -> Result<RuntimeStatusRecord, String> {
        let project = self
            .config_store
            .get_project(project_id)
            .ok_or_else(|| format!("Unknown project id: {project_id}"))?;

        let launch_request = self.resolve_launch_request(&project)?;
        self.runtime_manager.start_runtime(&launch_request)
    }

    pub fn stop_runtime(&self, project_id: &str) -> Result<RuntimeStatusRecord, String> {
        let project = self
            .config_store
            .get_project(project_id)
            .ok_or_else(|| format!("Unknown project id: {project_id}"))?;
        let reference = self.resolve_runtime_reference(&project)?;
        self.runtime_manager.stop_runtime(&reference)
    }

    pub fn get_runtime_status(&self, project_id: &str) -> Result<RuntimeStatusRecord, String> {
        let project = self
            .config_store
            .get_project(project_id)
            .ok_or_else(|| format!("Unknown project id: {project_id}"))?;
        let settings = self.config_store.get_settings();
        match self.resolve_runtime_reference(&project) {
            Ok(reference) => self.runtime_manager.get_runtime_status(&reference),
            Err(detail) => Ok(self.unresolved_runtime_status(&project, &settings, detail)),
        }
    }

    fn collect_runtime_statuses(
        &self,
        projects: &[ProjectRecord],
        settings: &ManagerSettings,
        installed_runtime: Option<&ManagedRuntimeRecord>,
    ) -> HashMap<String, RuntimeStatusRecord> {
        let mut statuses = HashMap::new();

        for project in projects {
            let status = match self.resolve_runtime_reference_with(project, settings, installed_runtime) {
                Ok(reference) => self
                    .runtime_manager
                    .get_runtime_status(&reference)
                    .unwrap_or_else(|error| self.unresolved_runtime_status(project, settings, error)),
                Err(detail) => self.unresolved_runtime_status(project, settings, detail),
            };
            statuses.insert(project.id.clone(), status);
        }

        statuses
    }

    fn resolve_launch_request(
        &self,
        project: &ProjectRecord,
    ) -> Result<RuntimeLaunchRequest, String> {
        let reference = self.resolve_runtime_reference(project)?;
        Ok(RuntimeLaunchRequest {
            project_path: project.project_path.clone(),
            reference,
        })
    }

    fn resolve_runtime_reference(
        &self,
        project: &ProjectRecord,
    ) -> Result<RuntimeReference, String> {
        let settings = self.config_store.get_settings();
        let installed = self.release_manager.get_installed_runtime(&settings)?;
        self.resolve_runtime_reference_with(project, &settings, installed.as_ref())
    }

    fn resolve_runtime_reference_with(
        &self,
        project: &ProjectRecord,
        settings: &ManagerSettings,
        installed_runtime: Option<&ManagedRuntimeRecord>,
    ) -> Result<RuntimeReference, String> {
        let workspace_dir = crate::config::display_path(&settings.workspace_root().join(&project.id));
        match &settings.global_runtime_source {
            RuntimeSource::Managed => {
                let runtime = installed_runtime
                    .ok_or_else(|| "No managed JavaLens runtime is installed. Download the latest release first.".to_string())?;

                Ok(RuntimeReference {
                    project_id: project.id.clone(),
                    assigned_port: project.assigned_port,
                    workspace_dir,
                    runtime_label: format!("Managed JavaLens {}", runtime.version),
                    resolved_jar_path: runtime.jar_path.clone(),
                })
            }
            RuntimeSource::LocalJar { jar_path } => Ok(RuntimeReference {
                project_id: project.id.clone(),
                assigned_port: project.assigned_port,
                workspace_dir,
                runtime_label: "Local JavaLens JAR".into(),
                resolved_jar_path: jar_path.clone(),
            }),
        }
    }

    fn unresolved_runtime_status(
        &self,
        project: &ProjectRecord,
        settings: &ManagerSettings,
        detail: String,
    ) -> RuntimeStatusRecord {
        let workspace_dir = crate::config::display_path(&settings.workspace_root().join(&project.id));
        RuntimeStatusRecord::unresolved(
            project.id.clone(),
            project.assigned_port,
            workspace_dir,
            settings.global_runtime_source.label(),
            detail,
        )
    }

    fn allocate_port(
        &self,
        settings: &ManagerSettings,
        excluding_project_id: Option<&str>,
        preferred: Option<u16>,
    ) -> Result<u16, String> {
        let used: HashSet<u16> = self
            .config_store
            .used_ports(excluding_project_id)
            .into_iter()
            .collect();

        let in_range = |port: u16| port >= settings.port_range_start && port <= settings.port_range_end;
        let available = |port: u16| in_range(port) && !used.contains(&port) && is_port_bind_available(port);

        if let Some(port) = preferred {
            if !in_range(port) {
                return Err(format!(
                    "Port {port} is outside permitted range {}-{}",
                    settings.port_range_start, settings.port_range_end
                ));
            }
            if !available(port) {
                return Err(format!("Port {port} is already in use or unavailable"));
            }
            return Ok(port);
        }

        self.suggest_next_port_for(settings, excluding_project_id)
            .ok_or_else(|| "No free port available in configured range".into())
    }

    fn suggest_next_port_for(
        &self,
        settings: &ManagerSettings,
        excluding_project_id: Option<&str>,
    ) -> Option<u16> {
        let used: HashSet<u16> = self
            .config_store
            .used_ports(excluding_project_id)
            .into_iter()
            .collect();
        (settings.port_range_start..=settings.port_range_end)
            .find(|port| !used.contains(port) && is_port_bind_available(*port))
    }
}

fn should_walk_entry(entry: &DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    if !entry.file_type().is_dir() {
        return true;
    }
    !matches!(
        name.as_ref(),
        ".git" | ".idea" | ".vscode" | "node_modules" | "target" | "build" | ".gradle" | ".metadata"
    )
}

fn is_ignored_candidate_path(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();

    if file_name == "External Plug-in Libraries"
        || file_name == "JRE System Library"
        || file_name.contains("BndtoolsJAREditorTempFiles")
    {
        return true;
    }

    for component in path.components() {
        let part = component.as_os_str().to_string_lossy();
        if part == ".metadata" || part == ".plugins" {
            return true;
        }
        if part.starts_with(".org.eclipse") || part.starts_with("org.eclipse.jdt.core.external.folders")
        {
            return true;
        }
    }

    false
}

fn detect_java_project_kind(path: &Path) -> Option<String> {
    let has = |name: &str| path.join(name).exists();
    let has_manifest = path.join("META-INF").join("MANIFEST.MF").exists();
    let has_java_src =
        path.join("src").join("main").join("java").exists() || path.join("src").join("test").join("java").exists();
    let has_build_files = has("pom.xml")
        || has("build.gradle")
        || has("build.gradle.kts")
        || has("settings.gradle")
        || has("settings.gradle.kts");
    let has_local_jars = has_local_jar_files(path);

    // Maven/Gradle entries must contain Java sources or local jar artifacts.
    if has_build_files && (has_java_src || has_local_jars) {
        return Some("maven-gradle".into());
    }

    // Eclipse/PDE must be an actual workspace project, not just a plugin/runtime folder.
    // Require .project and at least one Java/PDE signal.
    if has(".project")
        && (has(".classpath") || has_manifest || has("plugin.xml") || has("feature.xml") || has_java_src)
    {
        return Some("eclipse-pde".into());
    }

    None
}

fn has_local_jar_files(path: &Path) -> bool {
    for entry in WalkDir::new(path)
        .follow_links(false)
        .max_depth(4)
        .into_iter()
        .filter_entry(should_walk_entry)
    {
        let Ok(entry) = entry else {
            continue;
        };
        if entry.file_type().is_dir() && is_ignored_candidate_path(entry.path()) {
            continue;
        }
        if entry.file_type().is_file()
            && entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("jar"))
                .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

fn read_workspace_roots(workspace_file: &str) -> Result<Vec<PathBuf>, String> {
    let workspace_path = PathBuf::from(workspace_file);
    let workspace_dir = workspace_path
        .parent()
        .ok_or("workspace file has no parent directory")?;

    let contents = fs::read_to_string(&workspace_path)
        .map_err(|error| format!("failed to read workspace file {}: {error}", workspace_path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse workspace file {}: {error}", workspace_path.display()))?;

    let mut roots = Vec::new();
    if let Some(folders) = value.get("folders").and_then(|v| v.as_array()) {
        for folder in folders {
            if let Some(path) = folder.get("path").and_then(|v| v.as_str()) {
                let folder_path = PathBuf::from(path);
                if folder_path.is_absolute() {
                    roots.push(folder_path);
                } else {
                    roots.push(workspace_dir.join(folder_path));
                }
            }
        }
    }
    Ok(roots)
}

fn is_port_bind_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}
