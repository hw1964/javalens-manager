use crate::{
    config::{AddProjectInput, ProjectRecord, UpdateSettingsInput},
    manager_service::ManagerDashboard,
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
