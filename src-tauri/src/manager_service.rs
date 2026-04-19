use crate::{
    config::{
        AddProjectInput, BootstrapStatus, ConfigStore, ManagerSettings, ProjectRecord,
        RuntimeSource, UpdateSettingsInput,
    },
    release_manager::{ManagedRuntimeRecord, ReleaseManager, ReleaseStatus},
    runtime_manager::{
        RuntimeLaunchRequest, RuntimeManager, RuntimeReference, RuntimeStatusRecord,
    },
};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagerDashboard {
    pub bootstrap: BootstrapStatus,
    pub settings: ManagerSettings,
    pub release_status: ReleaseStatus,
    pub installed_runtime: Option<ManagedRuntimeRecord>,
    pub projects: Vec<ProjectRecord>,
    pub runtime_statuses: HashMap<String, RuntimeStatusRecord>,
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

        Ok(ManagerDashboard {
            bootstrap,
            settings,
            release_status,
            installed_runtime,
            projects,
            runtime_statuses,
        })
    }

    pub fn add_project(&self, input: AddProjectInput) -> Result<ProjectRecord, String> {
        self.config_store.add_project(input)
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
                    .ok_or_else(|| {
                        "No managed JavaLens runtime is installed. Download the latest release first."
                            .to_string()
                    })?;

                Ok(RuntimeReference {
                    project_id: project.id.clone(),
                    workspace_dir,
                    runtime_label: format!("Managed JavaLens {}", runtime.version),
                    resolved_jar_path: runtime.jar_path.clone(),
                })
            }
            RuntimeSource::LocalJar { jar_path } => Ok(RuntimeReference {
                project_id: project.id.clone(),
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
            workspace_dir,
            settings.global_runtime_source.label(),
            detail,
        )
    }
}
