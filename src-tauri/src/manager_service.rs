use crate::{
    config::{
        display_path, AddProjectInput, BootstrapStatus, ConfigStore, ManagerSettings,
        ProjectRecord, RuntimeSource, UpdateSettingsInput,
    },
    release_manager::{ManagedRuntimeRecord, ReleaseManager, ReleaseStatus},
    runtime_manager::{
        RuntimeLaunchRequest, RuntimeManager, RuntimePhase, RuntimeReference, RuntimeStatusRecord,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    process::{ChildStderr, ChildStdout, Command, Stdio},
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
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
    pub services_inventory: ServicesInventory,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServicesInventory {
    pub available: bool,
    pub services: Vec<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupSummary {
    pub target: String,
    pub deleted_files: usize,
    pub deleted_dirs: usize,
    pub failed_paths: Vec<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceProbeResult {
    pub ok: bool,
    pub services: Vec<ProbeServiceEntry>,
    pub detail: String,
    pub duration_ms: u128,
    pub raw_protocol_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeServiceEntry {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
struct ProbeRuntime {
    jar_path: String,
    runtime_label: String,
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
        let (installed_runtime, release_status) =
            self.release_manager.sync_with_settings(&mut settings)?;
        let settings = self.config_store.write_settings(settings)?;

        let projects = self.config_store.list_projects();
        let runtime_statuses =
            self.collect_runtime_statuses(&projects, &settings, installed_runtime.as_ref());
        let suggested_port = self.suggest_next_port_for(&settings, None);
        let services_inventory = self.get_services_inventory_with(installed_runtime.as_ref());

        Ok(ManagerDashboard {
            bootstrap,
            settings,
            release_status,
            installed_runtime,
            projects,
            runtime_statuses,
            suggested_port,
            services_inventory,
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

    pub fn update_project_port(
        &self,
        input: UpdateProjectPortInput,
    ) -> Result<ManagerDashboard, String> {
        let settings = self.config_store.get_settings();
        let assigned_port = self.allocate_port(
            &settings,
            Some(input.project_id.as_str()),
            Some(input.assigned_port),
        )?;
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
            return Err(format!(
                "Some runtimes failed to start: {}",
                errors.join(" | ")
            ));
        }

        self.load_dashboard()
    }

    pub fn stop_all_runtimes(&self) -> Result<ManagerDashboard, String> {
        let projects = self.config_store.list_projects();
        let mut errors = Vec::new();

        for project in projects {
            match self.resolve_runtime_reference(&project) {
                Ok(reference) => {
                    if let Err(error) = self.runtime_manager.stop_runtime(&reference) {
                        errors.push(format!("{}: {error}", project.name));
                    }
                }
                Err(error) => errors.push(format!("{}: {error}", project.name)),
            }
        }

        if !errors.is_empty() {
            return Err(format!(
                "Some runtimes failed to stop: {}",
                errors.join(" | ")
            ));
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

    pub fn get_services_inventory(&self) -> ServicesInventory {
        let settings = self.config_store.get_settings();
        let installed = self
            .release_manager
            .get_installed_runtime(&settings)
            .ok()
            .flatten();
        self.get_services_inventory_with(installed.as_ref())
    }

    pub fn clean_logs(&self) -> Result<CleanupSummary, String> {
        self.ensure_no_running_runtimes()?;
        let log_dir = self.config_store.paths().log_dir;
        let mut summary = cleanup_directory_contents(&log_dir)?;
        summary.target = "logs".into();
        Ok(summary)
    }

    pub fn clean_workspaces(&self) -> Result<CleanupSummary, String> {
        self.ensure_no_running_runtimes()?;
        let settings = self.config_store.get_settings();
        let workspace_root = settings.workspace_root();
        let mut summary = cleanup_directory_contents(&workspace_root)?;
        summary.target = "workspaces".into();
        Ok(summary)
    }

    pub fn clean_generated_data(&self) -> Result<CleanupSummary, String> {
        self.ensure_no_running_runtimes()?;
        let log_dir = self.config_store.paths().log_dir;
        let settings = self.config_store.get_settings();
        let workspace_root = settings.workspace_root();
        let logs = cleanup_directory_contents(&log_dir)?;
        let workspaces = cleanup_directory_contents(&workspace_root)?;

        let mut failed_paths = logs.failed_paths;
        failed_paths.extend(workspaces.failed_paths);
        let detail = if failed_paths.is_empty() {
            "Removed generated logs and workspaces.".to_string()
        } else {
            format!(
                "Removed generated data with {} partial failures.",
                failed_paths.len()
            )
        };

        Ok(CleanupSummary {
            target: "generatedData".into(),
            deleted_files: logs.deleted_files + workspaces.deleted_files,
            deleted_dirs: logs.deleted_dirs + workspaces.deleted_dirs,
            failed_paths,
            detail,
        })
    }

    pub fn probe_services(&self) -> Result<ServiceProbeResult, String> {
        let started_at = Instant::now();
        let settings = self.config_store.get_settings();
        let runtime = self.resolve_probe_runtime(&settings)?;
        let probe_workspace = settings.workspace_root().join(format!(
            "service-probe-{}",
            crate::config::current_timestamp_string()
        ));
        fs::create_dir_all(&probe_workspace).map_err(|error| {
            format!(
                "failed to create probe workspace {}: {error}",
                probe_workspace.display()
            )
        })?;

        let result = self.probe_services_with_runtime(&runtime, &probe_workspace, started_at);
        let _ = fs::remove_dir_all(&probe_workspace);
        Ok(result)
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
                    by_path
                        .entry(key.clone())
                        .or_insert_with(|| WorkspaceProjectCandidate {
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
            let status =
                match self.resolve_runtime_reference_with(project, settings, installed_runtime) {
                    Ok(reference) => self
                        .runtime_manager
                        .get_runtime_status(&reference)
                        .unwrap_or_else(|error| {
                            self.unresolved_runtime_status(project, settings, error)
                        }),
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
        let workspace_dir =
            crate::config::display_path(&settings.workspace_root().join(&project.id));
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
        let workspace_dir =
            crate::config::display_path(&settings.workspace_root().join(&project.id));
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

        let in_range =
            |port: u16| port >= settings.port_range_start && port <= settings.port_range_end;
        let available =
            |port: u16| in_range(port) && !used.contains(&port) && is_port_bind_available(port);

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

    fn ensure_no_running_runtimes(&self) -> Result<(), String> {
        let projects = self.config_store.list_projects();
        let mut active = Vec::new();

        for project in projects {
            let Ok(reference) = self.resolve_runtime_reference(&project) else {
                continue;
            };
            let Ok(status) = self.runtime_manager.get_runtime_status(&reference) else {
                continue;
            };
            if matches!(status.phase, RuntimePhase::Running | RuntimePhase::Starting) {
                active.push(project.name);
            }
        }

        if active.is_empty() {
            return Ok(());
        }

        Err(format!(
            "Stop running runtimes before cleanup: {}",
            active.join(", ")
        ))
    }

    fn get_services_inventory_with(
        &self,
        installed: Option<&ManagedRuntimeRecord>,
    ) -> ServicesInventory {
        let Some(runtime) = installed else {
            return ServicesInventory {
                available: false,
                services: Vec::new(),
                detail: "No managed runtime installed yet.".into(),
            };
        };

        let install_dir = PathBuf::from(&runtime.install_dir);
        let candidates = ["services.json", "tools.json", "manifest.json"];

        for file_name in candidates {
            let path = install_dir.join(file_name);
            if !path.exists() {
                continue;
            }

            match parse_services_from_json_file(&path) {
                Ok(services) if !services.is_empty() => {
                    return ServicesInventory {
                        available: true,
                        services,
                        detail: format!("Loaded service inventory from {}.", path.display()),
                    };
                }
                Ok(_) => {}
                Err(error) => {
                    return ServicesInventory {
                        available: false,
                        services: Vec::new(),
                        detail: format!(
                            "Service inventory file exists but could not be parsed: {} ({error})",
                            path.display()
                        ),
                    };
                }
            }
        }

        ServicesInventory {
            available: false,
            services: Vec::new(),
            detail: "Service inventory unavailable for this runtime package.".into(),
        }
    }

    fn resolve_probe_runtime(&self, settings: &ManagerSettings) -> Result<ProbeRuntime, String> {
        let installed = self.release_manager.get_installed_runtime(settings)?;
        let runtime = match &settings.global_runtime_source {
            RuntimeSource::Managed => {
                let runtime = installed.ok_or_else(|| {
                    "No managed JavaLens runtime is installed. Download latest first.".to_string()
                })?;
                ProbeRuntime {
                    jar_path: runtime.jar_path,
                    runtime_label: format!("Managed JavaLens {}", runtime.version),
                }
            }
            RuntimeSource::LocalJar { jar_path } => ProbeRuntime {
                jar_path: jar_path.clone(),
                runtime_label: "Local JavaLens JAR".into(),
            },
        };

        if !PathBuf::from(&runtime.jar_path).exists() {
            return Err(format!(
                "Configured JavaLens JAR does not exist: {}",
                runtime.jar_path
            ));
        }

        Ok(runtime)
    }

    fn probe_services_with_runtime(
        &self,
        runtime: &ProbeRuntime,
        probe_workspace: &Path,
        started_at: Instant,
    ) -> ServiceProbeResult {
        let mut command = Command::new("java");
        command
            .arg("-jar")
            .arg(&runtime.jar_path)
            .arg("-data")
            .arg(display_path(probe_workspace))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                return self.probe_failure(
                    format!("Failed to start JavaLens probe process: {error}"),
                    started_at,
                    None,
                );
            }
        };

        let stderr_tail = Arc::new(Mutex::new(Vec::<String>::new()));
        let stderr_handle = child
            .stderr
            .take()
            .map(|stderr| spawn_stderr_tail_reader(stderr, stderr_tail.clone()));

        let result = (|| {
            let mut stdin = child.stdin.take().ok_or_else(|| {
                "Probe process stdin was not available for MCP handshake".to_string()
            })?;
            let stdout = child.stdout.take().ok_or_else(|| {
                "Probe process stdout was not available for MCP handshake".to_string()
            })?;

            let responses = spawn_mcp_reader(stdout);

            let initialize_request = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "javalens-manager",
                        "version": "0.1.0"
                    }
                }
            });
            write_mcp_message(&mut stdin, &initialize_request)?;
            let initialize_response =
                wait_for_mcp_response(&responses, 1, Duration::from_secs(20))?;
            ensure_success_response(&initialize_response)?;

            let initialized_notification = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized",
                "params": {}
            });
            let _ = write_mcp_message(&mut stdin, &initialized_notification);

            let tools_list_request = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list",
                "params": {}
            });
            write_mcp_message(&mut stdin, &tools_list_request)?;
            let tools_list_response =
                wait_for_mcp_response(&responses, 2, Duration::from_secs(20))?;
            let mut services = extract_tool_entries(&tools_list_response)?;
            services.sort_by(|a, b| a.name.cmp(&b.name));
            services.dedup_by(|a, b| a.name == b.name);

            if services.is_empty() {
                return Ok(self.probe_failure(
                    format!(
                        "{} responded, but returned no tools for tools/list.",
                        runtime.runtime_label
                    ),
                    started_at,
                    None,
                ));
            }

            let invocation_detail =
                run_optional_invocation_check(&mut stdin, &responses, &services)
                    .map(|_| "Discovery + invocation check passed.".to_string())
                    .unwrap_or_else(|error| format!("Discovery only ({error})."));

            Ok(ServiceProbeResult {
                ok: true,
                services,
                detail: format!("Probe successful. {invocation_detail}"),
                duration_ms: started_at.elapsed().as_millis(),
                raw_protocol_error: None,
            })
        })();

        let _ = child.kill();
        let _ = child.wait();
        if let Some(handle) = stderr_handle {
            let _ = handle.join();
        }

        let stderr_snippet = collect_stderr_tail(&stderr_tail);
        match result {
            Ok(probe) => probe,
            Err(error) => {
                let detail = if let Some(stderr_tail) = stderr_snippet {
                    format!("Service probe failed: {error}. Runtime output: {stderr_tail}")
                } else {
                    format!("Service probe failed: {error}")
                };
                self.probe_failure(detail, started_at, Some(error))
            }
        }
    }

    fn probe_failure(
        &self,
        detail: String,
        started_at: Instant,
        raw_protocol_error: Option<String>,
    ) -> ServiceProbeResult {
        ServiceProbeResult {
            ok: false,
            services: Vec::new(),
            detail,
            duration_ms: started_at.elapsed().as_millis(),
            raw_protocol_error,
        }
    }
}

fn spawn_mcp_reader(stdout: ChildStdout) -> Receiver<Result<serde_json::Value, String>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let message = read_mcp_message(&mut reader);
            if tx.send(message.clone()).is_err() {
                break;
            }
            if message.is_err() {
                break;
            }
        }
    });
    rx
}

fn read_mcp_message(reader: &mut BufReader<ChildStdout>) -> Result<serde_json::Value, String> {
    let mut line = String::new();
    loop {
        line.clear();
        let read = reader
            .read_line(&mut line)
            .map_err(|error| format!("failed reading MCP response line: {error}"))?;
        if read == 0 {
            return Err("MCP stream closed before response was received".into());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !trimmed.starts_with('{') {
            return Err(format!(
                "received non-JSON output from JavaLens stdout: {trimmed}"
            ));
        }

        return serde_json::from_str::<serde_json::Value>(trimmed)
            .map_err(|error| format!("invalid MCP JSON payload: {error}"));
    }
}

fn spawn_stderr_tail_reader(
    stderr: ChildStderr,
    tail_lines: Arc<Mutex<Vec<String>>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(mut tail) = tail_lines.lock() {
                tail.push(line);
                if tail.len() > 12 {
                    let drain_count = tail.len() - 12;
                    tail.drain(0..drain_count);
                }
            }
        }
    })
}

fn collect_stderr_tail(tail_lines: &Arc<Mutex<Vec<String>>>) -> Option<String> {
    let Ok(lines) = tail_lines.lock() else {
        return None;
    };
    if lines.is_empty() {
        None
    } else {
        Some(lines.join(" | "))
    }
}

fn write_mcp_message(stdin: &mut impl Write, message: &serde_json::Value) -> Result<(), String> {
    let payload = serde_json::to_string(message)
        .map_err(|error| format!("failed serializing MCP message: {error}"))?;
    stdin
        .write_all(payload.as_bytes())
        .map_err(|error| format!("failed writing MCP message body: {error}"))?;
    stdin
        .write_all(b"\n")
        .map_err(|error| format!("failed writing MCP message newline: {error}"))?;
    stdin
        .flush()
        .map_err(|error| format!("failed flushing MCP message: {error}"))
}

fn wait_for_mcp_response(
    rx: &Receiver<Result<serde_json::Value, String>>,
    response_id: u64,
    timeout: Duration,
) -> Result<serde_json::Value, String> {
    let deadline = Instant::now() + timeout;
    loop {
        let now = Instant::now();
        if now >= deadline {
            return Err(format!(
                "timed out waiting for MCP response id {response_id}"
            ));
        }

        let remaining = deadline.saturating_duration_since(now);
        let message = rx
            .recv_timeout(remaining)
            .map_err(|_| format!("timed out waiting for MCP response id {response_id}"))??;
        if message_id_matches(&message, response_id) {
            return Ok(message);
        }
    }
}

fn message_id_matches(message: &serde_json::Value, response_id: u64) -> bool {
    message
        .get("id")
        .and_then(|id| id.as_u64())
        .map(|id| id == response_id)
        .unwrap_or(false)
}

fn ensure_success_response(response: &serde_json::Value) -> Result<(), String> {
    if let Some(error) = response.get("error") {
        return Err(format!("MCP returned error: {error}"));
    }
    if response.get("result").is_none() {
        return Err("MCP response did not include a result payload".into());
    }
    Ok(())
}

fn extract_tool_entries(response: &serde_json::Value) -> Result<Vec<ProbeServiceEntry>, String> {
    if let Some(error) = response.get("error") {
        return Err(format!("MCP tools/list returned error: {error}"));
    }

    let tools = response
        .get("result")
        .and_then(|result| result.get("tools"))
        .and_then(|tools| tools.as_array())
        .ok_or("MCP tools/list response did not include result.tools[]")?;

    let mut entries = Vec::new();
    for tool in tools {
        if let Some(name) = tool.get("name").and_then(|name| name.as_str()) {
            entries.push(ProbeServiceEntry {
                name: name.to_string(),
                description: tool
                    .get("description")
                    .and_then(|value| value.as_str())
                    .map(ToOwned::to_owned),
            });
        }
    }
    Ok(entries)
}

fn run_optional_invocation_check(
    stdin: &mut impl Write,
    responses: &Receiver<Result<serde_json::Value, String>>,
    services: &[ProbeServiceEntry],
) -> Result<(), String> {
    let Some(health_tool_name) = services.iter().find_map(|entry| {
        let lowered = entry.name.to_ascii_lowercase();
        if lowered == "health_check" || lowered == "healthcheck" || lowered == "health-check" {
            Some(entry.name.clone())
        } else {
            None
        }
    }) else {
        return Err("health check tool not advertised".into());
    };

    let call_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": health_tool_name,
            "arguments": {}
        }
    });
    write_mcp_message(stdin, &call_request)?;
    let call_response = wait_for_mcp_response(responses, 3, Duration::from_secs(20))?;
    ensure_success_response(&call_response)
}

fn cleanup_directory_contents(path: &Path) -> Result<CleanupSummary, String> {
    if !path.exists() {
        return Ok(CleanupSummary {
            target: display_path(path),
            deleted_files: 0,
            deleted_dirs: 0,
            failed_paths: Vec::new(),
            detail: "Nothing to clean.".into(),
        });
    }

    let mut deleted_files = 0usize;
    let mut deleted_dirs = 0usize;
    let mut failed_paths = Vec::new();
    let mut entries: Vec<PathBuf> = WalkDir::new(path)
        .min_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .map(|entry| entry.path().to_path_buf())
        .collect();
    entries.sort_by_key(|candidate| std::cmp::Reverse(candidate.components().count()));

    for entry in entries {
        let result = if entry.is_file() {
            fs::remove_file(&entry).map(|_| deleted_files += 1)
        } else if entry.is_dir() {
            fs::remove_dir(&entry).map(|_| deleted_dirs += 1)
        } else {
            Ok(())
        };

        if let Err(error) = result {
            failed_paths.push(format!("{} ({error})", entry.display()));
        }
    }

    let detail = if failed_paths.is_empty() {
        "Cleanup complete.".into()
    } else {
        format!(
            "Cleanup completed with {} partial failures.",
            failed_paths.len()
        )
    };

    Ok(CleanupSummary {
        target: display_path(path),
        deleted_files,
        deleted_dirs,
        failed_paths,
        detail,
    })
}

fn parse_services_from_json_file(path: &Path) -> Result<Vec<String>, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let value: serde_json::Value =
        serde_json::from_str(&contents).map_err(|error| format!("invalid JSON: {error}"))?;

    let mut services = Vec::new();
    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                if let Some(name) = item.as_str() {
                    services.push(name.to_string());
                } else if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                    services.push(name.to_string());
                } else if let Some(name) = item.get("toolName").and_then(|v| v.as_str()) {
                    services.push(name.to_string());
                }
            }
        }
        serde_json::Value::Object(map) => {
            if let Some(items) = map.get("tools").and_then(|v| v.as_array()) {
                for item in items {
                    if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                        services.push(name.to_string());
                    } else if let Some(name) = item.get("toolName").and_then(|v| v.as_str()) {
                        services.push(name.to_string());
                    }
                }
            }
        }
        _ => {}
    }

    services.sort();
    services.dedup();
    Ok(services)
}

fn should_walk_entry(entry: &DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    if !entry.file_type().is_dir() {
        return true;
    }
    !matches!(
        name.as_ref(),
        ".git"
            | ".idea"
            | ".vscode"
            | "node_modules"
            | "target"
            | "build"
            | ".gradle"
            | ".metadata"
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
        if part.starts_with(".org.eclipse")
            || part.starts_with("org.eclipse.jdt.core.external.folders")
        {
            return true;
        }
    }

    false
}

fn detect_java_project_kind(path: &Path) -> Option<String> {
    let has = |name: &str| path.join(name).exists();
    let has_manifest = path.join("META-INF").join("MANIFEST.MF").exists();
    let has_java_src = path.join("src").join("main").join("java").exists()
        || path.join("src").join("test").join("java").exists();
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
        && (has(".classpath")
            || has_manifest
            || has("plugin.xml")
            || has("feature.xml")
            || has_java_src)
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

    let contents = fs::read_to_string(&workspace_path).map_err(|error| {
        format!(
            "failed to read workspace file {}: {error}",
            workspace_path.display()
        )
    })?;
    let value: serde_json::Value = serde_json::from_str(&contents).map_err(|error| {
        format!(
            "failed to parse workspace file {}: {error}",
            workspace_path.display()
        )
    })?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_tool_entries_reads_standard_tools_list_shape() {
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "tools": [
                    { "name": "searchSymbols", "description": "Search symbols by query" },
                    { "name": "resolveReferences" }
                ]
            }
        });

        let tools = extract_tool_entries(&response).expect("tools/list should parse");
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "searchSymbols");
        assert_eq!(
            tools[0].description.as_deref(),
            Some("Search symbols by query")
        );
        assert_eq!(tools[1].name, "resolveReferences");
        assert_eq!(tools[1].description, None);
    }

    #[test]
    fn extract_tool_entries_surfaces_protocol_error_payload() {
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "error": {
                "code": -32601,
                "message": "Method not found"
            }
        });

        let error = extract_tool_entries(&response).expect_err("error payload should fail");
        assert!(error.contains("Method not found"));
    }
}
