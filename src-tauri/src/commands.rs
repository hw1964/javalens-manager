use crate::{
    config::{AddProjectInput, BootstrapStatus, ProjectRecord},
    runtime_manager::RuntimeStatusRecord,
    AppState,
};
use tauri::State;

#[tauri::command]
pub fn get_bootstrap_status(state: State<'_, AppState>) -> Result<BootstrapStatus, String> {
    Ok(state.config_store.bootstrap_status())
}

#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectRecord>, String> {
    Ok(state.config_store.list_projects())
}

#[tauri::command]
pub fn add_project(
    state: State<'_, AppState>,
    input: AddProjectInput,
) -> Result<ProjectRecord, String> {
    state.config_store.add_project(input)
}

#[tauri::command]
pub fn start_runtime(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<RuntimeStatusRecord, String> {
    let project = state
        .config_store
        .get_project(&project_id)
        .ok_or_else(|| format!("Unknown project id: {project_id}"))?;

    state.runtime_manager.start_runtime(&project)
}

#[tauri::command]
pub fn stop_runtime(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<RuntimeStatusRecord, String> {
    let project = state
        .config_store
        .get_project(&project_id)
        .ok_or_else(|| format!("Unknown project id: {project_id}"))?;

    state.runtime_manager.stop_runtime(&project)
}

#[tauri::command]
pub fn get_runtime_status(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<RuntimeStatusRecord, String> {
    let project = state
        .config_store
        .get_project(&project_id)
        .ok_or_else(|| format!("Unknown project id: {project_id}"))?;

    state.runtime_manager.get_runtime_status(&project)
}
