use crate::{
    config::{AddProjectInput, ProjectRecord, UpdateSettingsInput},
    manager_service::{
        CleanupSummary, ManagerDashboard, ServiceProbeResult, ServicesInventory,
        UpdateProjectPortInput, WorkspaceImportInput, WorkspaceImportResult,
        WorkspaceProjectCandidate,
    },
    runtime_manager::RuntimeStatusRecord,
    AppState,
};
use tauri::State;

#[tauri::command]
pub fn get_dashboard(state: State<'_, AppState>) -> Result<ManagerDashboard, String> {
    state.manager_service.load_dashboard()
}

#[tauri::command]
pub fn add_project(
    state: State<'_, AppState>,
    input: AddProjectInput,
) -> Result<ProjectRecord, String> {
    state.manager_service.add_project(input)
}

#[tauri::command]
pub fn suggest_next_port(state: State<'_, AppState>) -> Result<u16, String> {
    state.manager_service.suggest_next_port()
}

#[tauri::command]
pub fn update_project_port(
    state: State<'_, AppState>,
    input: UpdateProjectPortInput,
) -> Result<ManagerDashboard, String> {
    state.manager_service.update_project_port(input)
}

#[tauri::command]
pub fn delete_project(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<ManagerDashboard, String> {
    state.manager_service.delete_project(&project_id)
}

#[tauri::command]
pub fn start_all_runtimes(state: State<'_, AppState>) -> Result<ManagerDashboard, String> {
    state.manager_service.start_all_runtimes()
}

#[tauri::command]
pub fn stop_all_runtimes(state: State<'_, AppState>) -> Result<ManagerDashboard, String> {
    state.manager_service.stop_all_runtimes()
}

#[tauri::command]
pub fn delete_all_projects(state: State<'_, AppState>) -> Result<ManagerDashboard, String> {
    state.manager_service.delete_all_projects()
}

#[tauri::command]
pub fn discover_workspace_projects(
    state: State<'_, AppState>,
    workspace_file: String,
) -> Result<Vec<WorkspaceProjectCandidate>, String> {
    state
        .manager_service
        .discover_workspace_projects(&workspace_file)
}

#[tauri::command]
pub fn import_workspace_projects(
    state: State<'_, AppState>,
    input: WorkspaceImportInput,
) -> Result<WorkspaceImportResult, String> {
    state.manager_service.import_workspace_projects(input)
}

#[tauri::command]
pub fn update_settings(
    state: State<'_, AppState>,
    input: UpdateSettingsInput,
) -> Result<ManagerDashboard, String> {
    state.manager_service.update_settings(input)
}

#[tauri::command]
pub fn download_or_update_javalens(state: State<'_, AppState>) -> Result<ManagerDashboard, String> {
    state.manager_service.download_or_update_javalens()
}

#[tauri::command]
pub fn start_runtime(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<RuntimeStatusRecord, String> {
    state.manager_service.start_runtime(&project_id)
}

#[tauri::command]
pub fn stop_runtime(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<RuntimeStatusRecord, String> {
    state.manager_service.stop_runtime(&project_id)
}

#[tauri::command]
pub fn get_runtime_status(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<RuntimeStatusRecord, String> {
    state.manager_service.get_runtime_status(&project_id)
}

#[tauri::command]
pub fn get_services_inventory(state: State<'_, AppState>) -> Result<ServicesInventory, String> {
    Ok(state.manager_service.get_services_inventory())
}

#[tauri::command]
pub fn clean_logs(state: State<'_, AppState>) -> Result<CleanupSummary, String> {
    state.manager_service.clean_logs()
}

#[tauri::command]
pub fn clean_workspaces(state: State<'_, AppState>) -> Result<CleanupSummary, String> {
    state.manager_service.clean_workspaces()
}

#[tauri::command]
pub fn clean_generated_data(state: State<'_, AppState>) -> Result<CleanupSummary, String> {
    state.manager_service.clean_generated_data()
}

#[tauri::command]
pub fn probe_services(state: State<'_, AppState>) -> Result<ServiceProbeResult, String> {
    state.manager_service.probe_services()
}
