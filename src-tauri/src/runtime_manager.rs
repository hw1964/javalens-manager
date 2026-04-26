use crate::config::{display_path, AppPaths};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    path::Path,
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimePhase {
    Stopped,
    Starting,
    Running,
    Failed,
}

/// Status record for a project's runtime. Sprint 10 v0.10.4: multiple
/// projects sharing a `workspace_name` reflect the same underlying javalens
/// process — same PID, same workspace_dir, same log file. They differ only
/// in `project_id`. The frontend continues to read these per-project for
/// rendering, but the underlying process is shared.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatusRecord {
    pub project_id: String,
    pub phase: RuntimePhase,
    /// Sprint 10 v0.10.4: the logical workspace this project belongs to.
    /// All projects sharing this name run as one MCP service.
    pub workspace_name: String,
    pub transport: String,
    pub pid: Option<u32>,
    pub workspace_dir: String,
    pub log_path: String,
    pub runtime_label: String,
    pub resolved_jar_path: String,
    pub service_mode: String,
    pub detail: String,
}

impl RuntimeStatusRecord {
    pub fn unresolved(
        project_id: String,
        workspace_name: String,
        workspace_dir: String,
        runtime_label: String,
        detail: String,
    ) -> Self {
        Self {
            phase: RuntimePhase::Failed,
            workspace_name,
            transport: "stdio".into(),
            pid: None,
            log_path: String::new(),
            resolved_jar_path: String::new(),
            service_mode: "manager-process".into(),
            project_id,
            workspace_dir,
            runtime_label,
            detail,
        }
    }
}

/// Reference to a project's runtime — identifies which workspace process
/// the project belongs to. Built by manager_service from a `ProjectRecord`
/// + the resolved javalens runtime location.
#[derive(Debug, Clone)]
pub struct RuntimeReference {
    pub project_id: String,
    pub workspace_name: String,
    /// Eclipse `-data` directory for the workspace's javalens process.
    /// Lives at `<data_root>/workspaces/<workspace_name>/`. Manager_service
    /// writes `workspace.json` into here before spawn.
    pub workspace_dir: String,
    pub runtime_label: String,
    pub resolved_jar_path: String,
}

/// Launch request for one javalens spawn. Manager_service has already
/// written `<workspace_dir>/workspace.json` with the full project list of
/// the workspace before calling `start_runtime`.
#[derive(Debug, Clone)]
pub struct RuntimeLaunchRequest {
    pub project_path: String,
    pub reference: RuntimeReference,
}

#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub log_path: String,
}

/// One javalens process owned by the manager. Sprint 10 v0.10.4: shared
/// across every project whose `workspace_name` matches this entry's key.
struct ManagedRuntime {
    child: Child,
    started_at: Instant,
    log_path: String,
    /// Project IDs whose `workspace_name` made them members of this
    /// process. The process is killed when the last member leaves.
    members: HashSet<String>,
    /// Snapshot of the reference used to start the process — re-applied
    /// to per-project status records when other members query status.
    workspace_dir: String,
    runtime_label: String,
    resolved_jar_path: String,
}

pub struct RuntimeManager {
    paths: AppPaths,
    /// Sprint 10 v0.10.4: keyed by `workspace_name`, not `project_id`.
    handles: Mutex<HashMap<String, ManagedRuntime>>,
    /// Per-project snapshot cache. Multiple snapshots may point at the
    /// same `workspace_name` and reflect the same workspace process.
    snapshots: Mutex<HashMap<String, RuntimeStatusRecord>>,
}

impl RuntimeManager {
    pub fn new(paths: AppPaths) -> Self {
        let snapshots = read_runtime_state(&paths.runtime_state_file).unwrap_or_default();
        Self {
            paths,
            handles: Mutex::new(HashMap::new()),
            snapshots: Mutex::new(snapshots),
        }
    }

    /// Start (or join) the workspace's runtime for `launch_request.reference`.
    /// If the workspace's process is already running, this just adds the
    /// project as a member and returns the workspace's status. Otherwise
    /// spawns javalens. Caller (manager_service) must have written
    /// `workspace.json` into `workspace_dir` before calling.
    pub fn start_runtime(
        &self,
        launch_request: &RuntimeLaunchRequest,
    ) -> Result<RuntimeStatusRecord, String> {
        let reference = &launch_request.reference;

        // Fast path: workspace already running. Add membership, return
        // workspace's PID as this project's status.
        if let Some(status) = self.try_join_running_workspace(reference)? {
            return Ok(status);
        }

        self.paths.ensure_dirs()?;
        fs::create_dir_all(&reference.workspace_dir).map_err(|error| {
            format!(
                "failed to create workspace dir {}: {error}",
                reference.workspace_dir
            )
        })?;

        let command_spec = self.command_spec_for(launch_request);
        let log_path = command_spec.log_path.clone();
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|error| format!("failed to open {log_path}: {error}"))?;
        let stderr_file = log_file
            .try_clone()
            .map_err(|error| format!("failed to clone log file handle: {error}"))?;

        let mut command = Command::new(&command_spec.command);
        command.args(&command_spec.args);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::from(log_file));
        command.stderr(Stdio::from(stderr_file));

        for (key, value) in &command_spec.env {
            command.env(key, value);
        }

        let child = command.spawn().map_err(|error| {
            format!(
                "failed to launch JavaLens. Confirm Java and the resolved runtime path are valid: {error}"
            )
        })?;

        let pid = child.id();
        let status = RuntimeStatusRecord {
            project_id: reference.project_id.clone(),
            phase: RuntimePhase::Starting,
            workspace_name: reference.workspace_name.clone(),
            transport: "stdio".into(),
            pid: Some(pid),
            workspace_dir: reference.workspace_dir.clone(),
            log_path: log_path.clone(),
            runtime_label: reference.runtime_label.clone(),
            resolved_jar_path: reference.resolved_jar_path.clone(),
            service_mode: "manager-process".into(),
            detail: "Process launched. workspace.json drives in-process project loading.".into(),
        };

        let mut members = HashSet::new();
        members.insert(reference.project_id.clone());
        self.handles.lock().expect("runtime mutex poisoned").insert(
            reference.workspace_name.clone(),
            ManagedRuntime {
                child,
                started_at: Instant::now(),
                log_path,
                members,
                workspace_dir: reference.workspace_dir.clone(),
                runtime_label: reference.runtime_label.clone(),
                resolved_jar_path: reference.resolved_jar_path.clone(),
            },
        );
        self.persist_snapshot(status.clone())?;

        Ok(status)
    }

    /// Project leaves the workspace. If the workspace's process has no
    /// remaining members, it is killed. The caller (manager_service) is
    /// responsible for rewriting `workspace.json` so the still-running
    /// javalens (when there are remaining members) drops the leaving
    /// project from its in-memory state via the file watcher.
    pub fn stop_runtime(
        &self,
        reference: &RuntimeReference,
    ) -> Result<RuntimeStatusRecord, String> {
        let mut handles = self.handles.lock().expect("runtime mutex poisoned");

        let mut killed = false;
        if let Some(handle) = handles.get_mut(&reference.workspace_name) {
            handle.members.remove(&reference.project_id);
            if handle.members.is_empty() {
                if let Some(mut handle) = handles.remove(&reference.workspace_name) {
                    handle
                        .child
                        .kill()
                        .map_err(|error| format!("failed to stop JavaLens process: {error}"))?;
                    let _ = handle.child.wait();
                    killed = true;
                }
            }
        }
        drop(handles);

        let detail = if killed {
            "Workspace runtime stopped (last project left).".into()
        } else {
            "Project left the workspace; runtime continues for remaining members.".into()
        };
        let status = RuntimeStatusRecord {
            project_id: reference.project_id.clone(),
            phase: RuntimePhase::Stopped,
            workspace_name: reference.workspace_name.clone(),
            transport: "stdio".into(),
            pid: None,
            workspace_dir: reference.workspace_dir.clone(),
            log_path: self.default_log_path(&reference.workspace_name),
            runtime_label: reference.runtime_label.clone(),
            resolved_jar_path: reference.resolved_jar_path.clone(),
            service_mode: "manager-process".into(),
            detail,
        };

        self.persist_snapshot(status.clone())?;
        Ok(status)
    }

    /// Sprint 10 v0.10.4: stop the entire workspace process unconditionally.
    /// All members' snapshots become Stopped. Used by the "Stop workspace"
    /// button in the grouped Dashboard view.
    pub fn stop_workspace_runtime(&self, workspace_name: &str) -> Result<(), String> {
        let removed = {
            let mut handles = self.handles.lock().expect("runtime mutex poisoned");
            handles.remove(workspace_name)
        };
        if let Some(mut handle) = removed {
            let members = handle.members.clone();
            handle
                .child
                .kill()
                .map_err(|error| format!("failed to stop JavaLens process: {error}"))?;
            let _ = handle.child.wait();

            // Mark every member's snapshot as Stopped.
            for project_id in members {
                let snapshot = {
                    let snapshots = self
                        .snapshots
                        .lock()
                        .expect("runtime snapshot mutex poisoned");
                    snapshots.get(&project_id).cloned()
                };
                if let Some(mut s) = snapshot {
                    s.phase = RuntimePhase::Stopped;
                    s.pid = None;
                    s.detail = "Workspace runtime stopped.".into();
                    self.persist_snapshot(s)?;
                }
            }
        }
        Ok(())
    }

    pub fn get_runtime_status(
        &self,
        reference: &RuntimeReference,
    ) -> Result<RuntimeStatusRecord, String> {
        if let Some(status) = self.try_join_running_workspace_readonly(reference)? {
            return Ok(status);
        }

        Ok(self
            .snapshots
            .lock()
            .expect("runtime snapshot mutex poisoned")
            .get(&reference.project_id)
            .cloned()
            .unwrap_or_else(|| RuntimeStatusRecord {
                project_id: reference.project_id.clone(),
                phase: RuntimePhase::Stopped,
                workspace_name: reference.workspace_name.clone(),
                transport: "stdio".into(),
                pid: None,
                workspace_dir: reference.workspace_dir.clone(),
                log_path: self.default_log_path(&reference.workspace_name),
                runtime_label: reference.runtime_label.clone(),
                resolved_jar_path: reference.resolved_jar_path.clone(),
                service_mode: "manager-process".into(),
                detail: "Runtime has not been started yet.".into(),
            }))
    }

    /// Forcefully forget a project's runtime association. Removes from
    /// snapshots and from any workspace's member set. If the project was
    /// the last member, the workspace process is killed.
    pub fn remove_project_runtime(&self, project_id: &str) -> Result<(), String> {
        // Find which workspace (if any) hosts this project, and leave it.
        let host_workspace = {
            let handles = self.handles.lock().expect("runtime mutex poisoned");
            handles
                .iter()
                .find_map(|(ws, h)| {
                    if h.members.contains(project_id) {
                        Some(ws.clone())
                    } else {
                        None
                    }
                })
        };

        if let Some(ws) = host_workspace {
            let mut handles = self.handles.lock().expect("runtime mutex poisoned");
            if let Some(handle) = handles.get_mut(&ws) {
                handle.members.remove(project_id);
                if handle.members.is_empty() {
                    if let Some(mut handle) = handles.remove(&ws) {
                        let _ = handle.child.kill();
                        let _ = handle.child.wait();
                    }
                }
            }
        }

        let snapshots = {
            let mut snapshots = self
                .snapshots
                .lock()
                .expect("runtime snapshot mutex poisoned");
            snapshots.remove(project_id);
            snapshots.clone()
        };
        write_runtime_state(&self.paths.runtime_state_file, &snapshots)
    }

    pub fn command_spec_for(&self, launch_request: &RuntimeLaunchRequest) -> CommandSpec {
        let log_path = self.default_log_path(&launch_request.reference.workspace_name);

        // Sprint 10 v0.10.4: javalens reads its project list from
        // <workspace_dir>/workspace.json (written by manager_service).
        // No JAVA_PROJECT_PATH env var; that legacy single-project flow is
        // preserved in javalens-mcp v1.4.0 for direct manual launches but
        // the manager always uses the workspace.json contract.
        CommandSpec {
            command: "java".into(),
            args: vec![
                "-jar".into(),
                launch_request.reference.resolved_jar_path.clone(),
                "-data".into(),
                launch_request.reference.workspace_dir.clone(),
            ],
            env: vec![],
            log_path,
        }
    }

    /// If the workspace's process is already running, register the
    /// project as a member and return the workspace's PID as this
    /// project's status. Returns None if no process exists for the
    /// workspace yet (caller should spawn).
    fn try_join_running_workspace(
        &self,
        reference: &RuntimeReference,
    ) -> Result<Option<RuntimeStatusRecord>, String> {
        let mut handles = self.handles.lock().expect("runtime mutex poisoned");
        let Some(handle) = handles.get_mut(&reference.workspace_name) else {
            return Ok(None);
        };

        // Check if the process is still alive.
        if let Some(exit_status) = handle
            .child
            .try_wait()
            .map_err(|error| format!("failed to inspect JavaLens process state: {error}"))?
        {
            // Process has died — treat as no running workspace; caller
            // will spawn a new one.
            let detail = if exit_status.success() {
                "Previous workspace runtime exited cleanly; respawning.".into()
            } else {
                format!("Previous workspace runtime exited with status {exit_status}; respawning.")
            };
            handles.remove(&reference.workspace_name);
            drop(handles);

            // Persist a stopped snapshot for visibility, but signal "not
            // running" so the caller spawns afresh.
            let stopped = RuntimeStatusRecord {
                project_id: reference.project_id.clone(),
                phase: RuntimePhase::Stopped,
                workspace_name: reference.workspace_name.clone(),
                transport: "stdio".into(),
                pid: None,
                workspace_dir: reference.workspace_dir.clone(),
                log_path: self.default_log_path(&reference.workspace_name),
                runtime_label: reference.runtime_label.clone(),
                resolved_jar_path: reference.resolved_jar_path.clone(),
                service_mode: "manager-process".into(),
                detail,
            };
            self.persist_snapshot(stopped)?;
            return Ok(None);
        }

        handle.members.insert(reference.project_id.clone());
        let phase = if handle.started_at.elapsed() < Duration::from_secs(2) {
            RuntimePhase::Starting
        } else {
            RuntimePhase::Running
        };
        let status = RuntimeStatusRecord {
            project_id: reference.project_id.clone(),
            phase,
            workspace_name: reference.workspace_name.clone(),
            transport: "stdio".into(),
            pid: Some(handle.child.id()),
            workspace_dir: handle.workspace_dir.clone(),
            log_path: handle.log_path.clone(),
            runtime_label: handle.runtime_label.clone(),
            resolved_jar_path: handle.resolved_jar_path.clone(),
            service_mode: "manager-process".into(),
            detail: "Joined live workspace runtime; tools/list reflects current workspace.json."
                .into(),
        };
        drop(handles);
        self.persist_snapshot(status.clone())?;
        Ok(Some(status))
    }

    /// Same as try_join_running_workspace but does not add the project to
    /// the member set. Used by get_runtime_status, which mustn't have
    /// side effects on membership.
    fn try_join_running_workspace_readonly(
        &self,
        reference: &RuntimeReference,
    ) -> Result<Option<RuntimeStatusRecord>, String> {
        let mut handles = self.handles.lock().expect("runtime mutex poisoned");
        let Some(handle) = handles.get_mut(&reference.workspace_name) else {
            return Ok(None);
        };

        if let Some(_exit_status) = handle
            .child
            .try_wait()
            .map_err(|error| format!("failed to inspect JavaLens process state: {error}"))?
        {
            // Dead process — caller will see the persisted snapshot.
            handles.remove(&reference.workspace_name);
            return Ok(None);
        }

        let phase = if handle.started_at.elapsed() < Duration::from_secs(2) {
            RuntimePhase::Starting
        } else {
            RuntimePhase::Running
        };
        let status = RuntimeStatusRecord {
            project_id: reference.project_id.clone(),
            phase,
            workspace_name: reference.workspace_name.clone(),
            transport: "stdio".into(),
            pid: Some(handle.child.id()),
            workspace_dir: handle.workspace_dir.clone(),
            log_path: handle.log_path.clone(),
            runtime_label: handle.runtime_label.clone(),
            resolved_jar_path: handle.resolved_jar_path.clone(),
            service_mode: "manager-process".into(),
            detail: "Live workspace runtime.".into(),
        };
        Ok(Some(status))
    }

    fn persist_snapshot(&self, status: RuntimeStatusRecord) -> Result<(), String> {
        let snapshots = {
            let mut snapshots = self
                .snapshots
                .lock()
                .expect("runtime snapshot mutex poisoned");
            snapshots.insert(status.project_id.clone(), status);
            snapshots.clone()
        };

        write_runtime_state(&self.paths.runtime_state_file, &snapshots)
    }

    fn default_log_path(&self, workspace_name: &str) -> String {
        // Sprint 10 v0.10.4: log path keyed by workspace_name (one log per
        // workspace process), not per project_id.
        display_path(&self.paths.log_dir.join(format!("{workspace_name}.log")))
    }
}

fn read_runtime_state(path: &Path) -> Result<HashMap<String, RuntimeStatusRecord>, String> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read runtime state {}: {error}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse runtime state {}: {error}", path.display()))
}

fn write_runtime_state(
    path: &Path,
    snapshots: &HashMap<String, RuntimeStatusRecord>,
) -> Result<(), String> {
    let json = serde_json::to_string_pretty(snapshots).map_err(|error| {
        format!(
            "failed to serialize runtime state {}: {error}",
            path.display()
        )
    })?;
    fs::write(path, format!("{json}\n"))
        .map_err(|error| format!("failed to write runtime state {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppPaths;
    use std::path::PathBuf;

    fn fake_paths() -> AppPaths {
        AppPaths {
            config_dir: PathBuf::from("/tmp/javalens-manager/config"),
            state_dir: PathBuf::from("/tmp/javalens-manager/state"),
            cache_dir: PathBuf::from("/tmp/javalens-manager/cache"),
            projects_file: PathBuf::from("/tmp/javalens-manager/config/projects.json"),
            settings_file: PathBuf::from("/tmp/javalens-manager/config/settings.json"),
            runtime_state_file: PathBuf::from("/tmp/javalens-manager/state/runtime-state.json"),
            default_data_root: PathBuf::from("/tmp/javalens-manager/cache"),
            log_dir: PathBuf::from("/tmp/javalens-manager/state/logs"),
        }
    }

    fn fake_launch_request() -> RuntimeLaunchRequest {
        RuntimeLaunchRequest {
            project_path: "/projects/example-service".into(),
            reference: RuntimeReference {
                project_id: "example-service-1".into(),
                workspace_name: "test-ws".into(),
                workspace_dir: "/cache/javalens/test-ws".into(),
                runtime_label: "Managed JavaLens 1.4.0".into(),
                resolved_jar_path: "/tools/javalens/javalens.jar".into(),
            },
        }
    }

    #[test]
    fn command_spec_uses_workspace_dir_and_no_env_var() {
        let manager = RuntimeManager::new(fake_paths());
        let launch_request = fake_launch_request();

        let spec = manager.command_spec_for(&launch_request);

        assert_eq!(spec.command, "java");
        assert_eq!(
            spec.args,
            vec![
                "-jar",
                "/tools/javalens/javalens.jar",
                "-data",
                "/cache/javalens/test-ws"
            ]
        );
        // Sprint 10 v0.10.4: no JAVA_PROJECT_PATH — workspace.json drives
        // project loading inside javalens.
        assert!(spec.env.is_empty());
        assert!(spec.log_path.ends_with("test-ws.log"));
    }

    #[test]
    fn unresolved_runtime_status_carries_runtime_label() {
        let status = RuntimeStatusRecord::unresolved(
            "project-1".into(),
            "test-ws".into(),
            "/tmp/workspace".into(),
            "Managed JavaLens 1.4.0".into(),
            "Missing runtime".into(),
        );

        assert!(matches!(status.phase, RuntimePhase::Failed));
        assert_eq!(status.workspace_name, "test-ws");
        assert_eq!(status.runtime_label, "Managed JavaLens 1.4.0");
        assert_eq!(status.detail, "Missing runtime");
    }
}
