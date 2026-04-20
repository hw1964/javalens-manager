mod commands;
mod config;
mod manager_service;
mod release_manager;
mod runtime_manager;

use config::ConfigStore;
use manager_service::ManagerService;
use release_manager::ReleaseManager;
use runtime_manager::RuntimeManager;

pub struct AppState {
    pub manager_service: ManagerService,
}

pub fn run() {
    let config_store = ConfigStore::new().expect("failed to initialize config store");
    let release_manager =
        ReleaseManager::new().expect("failed to initialize release manager");
    let runtime_manager = RuntimeManager::new(config_store.paths());
    let manager_service = ManagerService::new(config_store, release_manager, runtime_manager);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState { manager_service })
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard,
            commands::add_project,
            commands::suggest_next_port,
            commands::update_project_port,
            commands::delete_project,
            commands::start_all_runtimes,
            commands::stop_all_runtimes,
            commands::delete_all_projects,
            commands::discover_workspace_projects,
            commands::import_workspace_projects,
            commands::update_settings,
            commands::download_or_update_javalens,
            commands::start_runtime,
            commands::stop_runtime,
            commands::get_runtime_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running javalens-manager");
}
