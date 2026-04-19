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
    pub installed_runtimes: Vec<ManagedRuntimeRecord>,
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
        let (installed_runtimes, release_status) =
            self.release_manager.sync_with_settings(&mut settings)?;
        let settings = self.config_store.write_settings(settings)?;

        let projects = self.config_store.list_projects();
        let runtime_statuses =
            self.collect_runtime_statuses(&projects, &settings, &installed_runtimes);

        Ok(ManagerDashboard {
            bootstrap,
            settings,
            release_status,
            installed_runtimes,
            projects,
            runtime_statuses,
        })
    }

    pub fn add_project(&self, input: AddProjectInput) -> Result<ProjectRecord, String> {
        if let RuntimeSource::Managed { version } = &input.runtime_source {
            let settings = self.config_store.get_settings();
            let installed = self.release_manager.list_installed_runtimes(&settings)?;
            if !installed.iter().any(|runtime| runtime.version == *version) {
                return Err(format!(
                    "Managed JavaLens runtime {version} is not installed yet. Download it first."
                ));
            }
        }

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
        match self.resolve_runtime_reference(&project) {
            Ok(reference) => self.runtime_manager.get_runtime_status(&reference),
            Err(detail) => Ok(self.unresolved_runtime_status(&project, detail)),
        }
    }

    fn collect_runtime_statuses(
        &self,
        projects: &[ProjectRecord],
        settings: &ManagerSettings,
        installed_runtimes: &[ManagedRuntimeRecord],
    ) -> HashMap<String, RuntimeStatusRecord> {
        let mut statuses = HashMap::new();

        for project in projects {
            let status =
                match self.resolve_runtime_reference_with(project, settings, installed_runtimes) {
                    Ok(reference) => self
                        .runtime_manager
                        .get_runtime_status(&reference)
                        .unwrap_or_else(|error| self.unresolved_runtime_status(project, error)),
                    Err(detail) => self.unresolved_runtime_status(project, detail),
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
        let installed = self.release_manager.list_installed_runtimes(&settings)?;
        self.resolve_runtime_reference_with(project, &settings, &installed)
    }

    fn resolve_runtime_reference_with(
        &self,
        project: &ProjectRecord,
        settings: &ManagerSettings,
        installed_runtimes: &[ManagedRuntimeRecord],
    ) -> Result<RuntimeReference, String> {
        match &project.runtime_source {
            RuntimeSource::Managed { version } => {
                let selected_version = if version.trim().is_empty() {
                    settings
                        .default_managed_runtime_version
                        .clone()
                        .ok_or("No default managed JavaLens runtime version is selected yet")?
                } else {
                    version.clone()
                };
                let runtime = installed_runtimes
                    .iter()
                    .find(|runtime| runtime.version == selected_version)
                    .ok_or_else(|| {
                        format!(
                            "Managed JavaLens runtime {selected_version} is not installed. Download the latest release first."
                        )
                    })?;

                Ok(RuntimeReference {
                    project_id: project.id.clone(),
                    workspace_dir: project.workspace_dir.clone(),
                    runtime_label: format!("Managed JavaLens {}", runtime.version),
                    resolved_jar_path: runtime.jar_path.clone(),
                })
            }
            RuntimeSource::LocalJar { jar_path } => Ok(RuntimeReference {
                project_id: project.id.clone(),
                workspace_dir: project.workspace_dir.clone(),
                runtime_label: "Local JavaLens JAR".into(),
                resolved_jar_path: jar_path.clone(),
            }),
        }
    }

    fn unresolved_runtime_status(
        &self,
        project: &ProjectRecord,
        detail: String,
    ) -> RuntimeStatusRecord {
        RuntimeStatusRecord::unresolved(
            project.id.clone(),
            project.workspace_dir.clone(),
            project.runtime_source.label(),
            detail,
        )
    }
}
