use crate::config::{display_path, AppPaths, ProjectRecord};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimePhase {
    Stopped,
    Starting,
    Running,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatusRecord {
    pub project_id: String,
    pub phase: RuntimePhase,
    pub transport: String,
    pub pid: Option<u32>,
    pub workspace_dir: String,
    pub log_path: String,
    pub detail: String,
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
        Self {
            paths,
            handles: Mutex::new(HashMap::new()),
            snapshots: Mutex::new(HashMap::new()),
        }
    }

    pub fn start_runtime(&self, project: &ProjectRecord) -> Result<RuntimeStatusRecord, String> {
        if let Some(status) = self.try_get_active_status(project)? {
            return Ok(status);
        }

        self.paths.ensure_dirs()?;
        fs::create_dir_all(&project.workspace_dir).map_err(|error| {
            format!(
                "failed to create workspace dir {}: {error}",
                project.workspace_dir
            )
        })?;

        let command_spec = self.command_spec_for(project);
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
                "failed to launch JavaLens. Confirm Java, the JAR path, and workspace path are valid: {error}"
            )
        })?;

        let status = RuntimeStatusRecord {
            project_id: project.id.clone(),
            phase: RuntimePhase::Starting,
            transport: "stdio".into(),
            pid: Some(child.id()),
            workspace_dir: project.workspace_dir.clone(),
            log_path: log_path.clone(),
            detail:
                "Process launched. First slice uses process liveness; MCP health_check comes next."
                    .into(),
        };

        self.handles.lock().expect("runtime mutex poisoned").insert(
            project.id.clone(),
            ManagedRuntime {
                child,
                started_at: Instant::now(),
                log_path,
            },
        );
        self.snapshots
            .lock()
            .expect("runtime snapshot mutex poisoned")
            .insert(project.id.clone(), status.clone());

        Ok(status)
    }

    pub fn stop_runtime(&self, project: &ProjectRecord) -> Result<RuntimeStatusRecord, String> {
        if let Some(mut handle) = self
            .handles
            .lock()
            .expect("runtime mutex poisoned")
            .remove(&project.id)
        {
            handle
                .child
                .kill()
                .map_err(|error| format!("failed to stop JavaLens process: {error}"))?;
            let _ = handle.child.wait();
        }

        let status = RuntimeStatusRecord {
            project_id: project.id.clone(),
            phase: RuntimePhase::Stopped,
            transport: "stdio".into(),
            pid: None,
            workspace_dir: project.workspace_dir.clone(),
            log_path: self.default_log_path(project),
            detail: "Runtime stopped.".into(),
        };

        self.snapshots
            .lock()
            .expect("runtime snapshot mutex poisoned")
            .insert(project.id.clone(), status.clone());

        Ok(status)
    }

    pub fn get_runtime_status(
        &self,
        project: &ProjectRecord,
    ) -> Result<RuntimeStatusRecord, String> {
        if let Some(status) = self.try_get_active_status(project)? {
            return Ok(status);
        }

        Ok(self
            .snapshots
            .lock()
            .expect("runtime snapshot mutex poisoned")
            .get(&project.id)
            .cloned()
            .unwrap_or_else(|| RuntimeStatusRecord {
                project_id: project.id.clone(),
                phase: RuntimePhase::Stopped,
                transport: "stdio".into(),
                pid: None,
                workspace_dir: project.workspace_dir.clone(),
                log_path: self.default_log_path(project),
                detail: "Runtime has not been started yet.".into(),
            }))
    }

    pub fn command_spec_for(&self, project: &ProjectRecord) -> CommandSpec {
        let log_path = self.default_log_path(project);

        CommandSpec {
            command: "java".into(),
            args: vec![
                "-jar".into(),
                project.javalens_jar_path.clone(),
                "-data".into(),
                project.workspace_dir.clone(),
            ],
            env: vec![("JAVA_PROJECT_PATH".into(), project.project_path.clone())],
            log_path,
        }
    }

    fn try_get_active_status(
        &self,
        project: &ProjectRecord,
    ) -> Result<Option<RuntimeStatusRecord>, String> {
        let mut finished_status: Option<RuntimeStatusRecord> = None;
        let mut running_status: Option<RuntimeStatusRecord> = None;

        {
            let mut handles = self.handles.lock().expect("runtime mutex poisoned");

            if let Some(handle) = handles.get_mut(&project.id) {
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
                        project_id: project.id.clone(),
                        phase: if exit_status.success() {
                            RuntimePhase::Stopped
                        } else {
                            RuntimePhase::Failed
                        },
                        transport: "stdio".into(),
                        pid: None,
                        workspace_dir: project.workspace_dir.clone(),
                        log_path: handle.log_path.clone(),
                        detail,
                    });
                } else {
                    let phase = if handle.started_at.elapsed() < Duration::from_secs(2) {
                        RuntimePhase::Starting
                    } else {
                        RuntimePhase::Running
                    };

                    running_status = Some(RuntimeStatusRecord {
                        project_id: project.id.clone(),
                        phase,
                        transport: "stdio".into(),
                        pid: Some(handle.child.id()),
                        workspace_dir: project.workspace_dir.clone(),
                        log_path: handle.log_path.clone(),
                        detail:
                            "Process is alive. MCP health_check will be added after the first slice."
                                .into(),
                    });
                }
            }
        }

        if let Some(status) = finished_status {
            self.handles
                .lock()
                .expect("runtime mutex poisoned")
                .remove(&project.id);
            self.snapshots
                .lock()
                .expect("runtime snapshot mutex poisoned")
                .insert(project.id.clone(), status.clone());
            return Ok(Some(status));
        }

        if let Some(status) = running_status {
            self.snapshots
                .lock()
                .expect("runtime snapshot mutex poisoned")
                .insert(project.id.clone(), status.clone());
            return Ok(Some(status));
        }

        Ok(None)
    }

    fn default_log_path(&self, project: &ProjectRecord) -> String {
        display_path(&self.paths.log_dir.join(format!("{}.log", project.id)))
    }
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
            config_file: PathBuf::from("/tmp/javalens-manager/config/projects.json"),
            workspace_root: PathBuf::from("/tmp/javalens-manager/cache/workspaces"),
            log_dir: PathBuf::from("/tmp/javalens-manager/state/logs"),
        }
    }

    fn fake_project() -> ProjectRecord {
        ProjectRecord {
            id: "example-service-1".into(),
            name: "Example Service".into(),
            project_path: "/projects/example-service".into(),
            javalens_jar_path: "/tools/javalens/javalens.jar".into(),
            workspace_dir: "/cache/javalens/example-service".into(),
        }
    }

    #[test]
    fn command_spec_matches_documented_launch_contract() {
        let manager = RuntimeManager::new(fake_paths());
        let project = fake_project();

        let spec = manager.command_spec_for(&project);

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
}
