mod commands;
mod config;
mod runtime_manager;

use config::ConfigStore;
use runtime_manager::RuntimeManager;

pub struct AppState {
    pub config_store: ConfigStore,
    pub runtime_manager: RuntimeManager,
}

pub fn run() {
    let config_store = ConfigStore::new().expect("failed to initialize config store");
    let runtime_manager = RuntimeManager::new(config_store.paths());

    tauri::Builder::default()
        .manage(AppState {
            config_store,
            runtime_manager,
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap_status,
            commands::list_projects,
            commands::add_project,
            commands::start_runtime,
            commands::stop_runtime,
            commands::get_runtime_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running javalens-manager");
}
