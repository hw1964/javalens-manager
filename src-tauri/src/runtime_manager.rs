use crate::config::{display_path, AppPaths};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatusRecord {
    pub project_id: String,
    pub phase: RuntimePhase,
    pub assigned_port: u16,
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
        assigned_port: u16,
        workspace_dir: String,
        runtime_label: String,
        detail: String,
    ) -> Self {
        Self {
            phase: RuntimePhase::Failed,
            assigned_port,
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

#[derive(Debug, Clone)]
pub struct RuntimeReference {
    pub project_id: String,
    pub assigned_port: u16,
    pub workspace_dir: String,
    pub runtime_label: String,
    pub resolved_jar_path: String,
}

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

struct ManagedRuntime {
    child: Child,
    started_at: Instant,
    log_path: String,
}

pub struct RuntimeManager {
    paths: AppPaths,
    handles: Mutex<HashMap<String, ManagedRuntime>>,
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

    pub fn start_runtime(
        &self,
        launch_request: &RuntimeLaunchRequest,
    ) -> Result<RuntimeStatusRecord, String> {
        if let Some(status) = self.try_get_active_status(&launch_request.reference)? {
            return Ok(status);
        }

        self.paths.ensure_dirs()?;
        fs::create_dir_all(&launch_request.reference.workspace_dir).map_err(|error| {
            format!(
                "failed to create workspace dir {}: {error}",
                launch_request.reference.workspace_dir
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

        let status = RuntimeStatusRecord {
            project_id: launch_request.reference.project_id.clone(),
            phase: RuntimePhase::Starting,
            assigned_port: launch_request.reference.assigned_port,
            transport: "stdio".into(),
            pid: Some(child.id()),
            workspace_dir: launch_request.reference.workspace_dir.clone(),
            log_path: log_path.clone(),
            runtime_label: launch_request.reference.runtime_label.clone(),
            resolved_jar_path: launch_request.reference.resolved_jar_path.clone(),
            service_mode: "manager-process".into(),
            detail: "Process launched. Next slice can add semantic health probes.".into(),
        };

        self.handles.lock().expect("runtime mutex poisoned").insert(
            launch_request.reference.project_id.clone(),
            ManagedRuntime {
                child,
                started_at: Instant::now(),
                log_path,
            },
        );
        self.persist_snapshot(status.clone())?;

        Ok(status)
    }

    pub fn stop_runtime(
        &self,
        reference: &RuntimeReference,
    ) -> Result<RuntimeStatusRecord, String> {
        if let Some(mut handle) = self
            .handles
            .lock()
            .expect("runtime mutex poisoned")
            .remove(&reference.project_id)
        {
            handle
                .child
                .kill()
                .map_err(|error| format!("failed to stop JavaLens process: {error}"))?;
            let _ = handle.child.wait();
        }

        let status = RuntimeStatusRecord {
            project_id: reference.project_id.clone(),
            phase: RuntimePhase::Stopped,
            assigned_port: reference.assigned_port,
            transport: "stdio".into(),
            pid: None,
            workspace_dir: reference.workspace_dir.clone(),
            log_path: self.default_log_path(&reference.project_id),
            runtime_label: reference.runtime_label.clone(),
            resolved_jar_path: reference.resolved_jar_path.clone(),
            service_mode: "manager-process".into(),
            detail: "Runtime stopped.".into(),
        };

        self.persist_snapshot(status.clone())?;
        Ok(status)
    }

    pub fn get_runtime_status(
        &self,
        reference: &RuntimeReference,
    ) -> Result<RuntimeStatusRecord, String> {
        if let Some(status) = self.try_get_active_status(reference)? {
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
                assigned_port: reference.assigned_port,
                transport: "stdio".into(),
                pid: None,
                workspace_dir: reference.workspace_dir.clone(),
                log_path: self.default_log_path(&reference.project_id),
                runtime_label: reference.runtime_label.clone(),
                resolved_jar_path: reference.resolved_jar_path.clone(),
                service_mode: "manager-process".into(),
                detail: "Runtime has not been started yet.".into(),
            }))
    }

    pub fn remove_project_runtime(&self, project_id: &str) -> Result<(), String> {
        if let Some(mut handle) = self
            .handles
            .lock()
            .expect("runtime mutex poisoned")
            .remove(project_id)
        {
            let _ = handle.child.kill();
            let _ = handle.child.wait();
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
        let log_path = self.default_log_path(&launch_request.reference.project_id);

        CommandSpec {
            command: "java".into(),
            args: vec![
                "-jar".into(),
                launch_request.reference.resolved_jar_path.clone(),
                "-data".into(),
                launch_request.reference.workspace_dir.clone(),
            ],
            env: vec![(
                "JAVA_PROJECT_PATH".into(),
                launch_request.project_path.clone(),
            )],
            log_path,
        }
    }

    fn try_get_active_status(
        &self,
        reference: &RuntimeReference,
    ) -> Result<Option<RuntimeStatusRecord>, String> {
        let mut finished_status: Option<RuntimeStatusRecord> = None;
        let mut running_status: Option<RuntimeStatusRecord> = None;

        {
            let mut handles = self.handles.lock().expect("runtime mutex poisoned");

            if let Some(handle) = handles.get_mut(&reference.project_id) {
                if let Some(exit_status) = handle
                    .child
                    .try_wait()
                    .map_err(|error| format!("failed to inspect JavaLens process state: {error}"))?
                {
                    let detail = if exit_status.success() {
                        "Runtime exited cleanly.".into()
                    } else {
                        format!("Runtime exited with status {exit_status}.")
                    };

                    finished_status = Some(RuntimeStatusRecord {
                        project_id: reference.project_id.clone(),
                        phase: if exit_status.success() {
                            RuntimePhase::Stopped
                        } else {
                            RuntimePhase::Failed
                        },
                        assigned_port: reference.assigned_port,
                        transport: "stdio".into(),
                        pid: None,
                        workspace_dir: reference.workspace_dir.clone(),
                        log_path: handle.log_path.clone(),
                        runtime_label: reference.runtime_label.clone(),
                        resolved_jar_path: reference.resolved_jar_path.clone(),
                        service_mode: "manager-process".into(),
                        detail,
                    });
                } else {
                    let phase = if handle.started_at.elapsed() < Duration::from_secs(2) {
                        RuntimePhase::Starting
                    } else {
                        RuntimePhase::Running
                    };

                    running_status = Some(RuntimeStatusRecord {
                        project_id: reference.project_id.clone(),
                        phase,
                        assigned_port: reference.assigned_port,
                        transport: "stdio".into(),
                        pid: Some(handle.child.id()),
                        workspace_dir: reference.workspace_dir.clone(),
                        log_path: handle.log_path.clone(),
                        runtime_label: reference.runtime_label.clone(),
                        resolved_jar_path: reference.resolved_jar_path.clone(),
                        service_mode: "manager-process".into(),
                        detail:
                            "Process is alive. Upstream health_check is still deferred to a later slice."
                                .into(),
                    });
                }
            }
        }

        if let Some(status) = finished_status {
            self.handles
                .lock()
                .expect("runtime mutex poisoned")
                .remove(&reference.project_id);
            self.persist_snapshot(status.clone())?;
            return Ok(Some(status));
        }

        if let Some(status) = running_status {
            self.persist_snapshot(status.clone())?;
            return Ok(Some(status));
        }

        Ok(None)
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

    fn default_log_path(&self, project_id: &str) -> String {
        display_path(&self.paths.log_dir.join(format!("{project_id}.log")))
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
                assigned_port: 11100,
                workspace_dir: "/cache/javalens/example-service".into(),
                runtime_label: "Managed JavaLens 1.2.0".into(),
                resolved_jar_path: "/tools/javalens/javalens.jar".into(),
            },
        }
    }

    #[test]
    fn command_spec_matches_documented_launch_contract() {
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
                "/cache/javalens/example-service"
            ]
        );
        assert_eq!(
            spec.env,
            vec![(
                "JAVA_PROJECT_PATH".into(),
                "/projects/example-service".into()
            )]
        );
        assert!(spec.log_path.ends_with("example-service-1.log"));
    }

    #[test]
    fn unresolved_runtime_status_carries_runtime_label() {
        let status = RuntimeStatusRecord::unresolved(
            "project-1".into(),
            11100,
            "/tmp/workspace".into(),
            "Managed JavaLens 1.2.0".into(),
            "Missing runtime".into(),
        );

        assert!(matches!(status.phase, RuntimePhase::Failed));
        assert_eq!(status.runtime_label, "Managed JavaLens 1.2.0");
        assert_eq!(status.detail, "Missing runtime");
    }
}
