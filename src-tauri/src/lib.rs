mod commands;
mod config;
mod manager_service;
mod release_manager;
mod runtime_manager;

use config::ConfigStore;
use manager_service::ManagerService;
use release_manager::ReleaseManager;
use runtime_manager::RuntimeManager;
use serde::Serialize;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, WindowEvent,
};

pub struct AppState {
    pub manager_service: ManagerService,
}

const TRAY_ICON_SIZE: u32 = 32;

#[derive(Clone, Copy)]
enum TrayIconVariant {
    JCircle,
    CoffeeCircle,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct QuitPromptEvent {
    source: String,
    running_services: usize,
    tray_enabled: bool,
}

fn selected_tray_icon_variant() -> TrayIconVariant {
    match std::env::var("JAVALENS_TRAY_ICON")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "coffee" | "cup" => TrayIconVariant::CoffeeCircle,
        _ => TrayIconVariant::JCircle,
    }
}

fn build_tray_icon(variant: TrayIconVariant) -> Image<'static> {
    let mut rgba = vec![0u8; (TRAY_ICON_SIZE * TRAY_ICON_SIZE * 4) as usize];
    draw_base_circle(&mut rgba);
    match variant {
        TrayIconVariant::JCircle => draw_j_glyph(&mut rgba),
        TrayIconVariant::CoffeeCircle => draw_coffee_glyph(&mut rgba),
    }
    Image::new_owned(rgba, TRAY_ICON_SIZE, TRAY_ICON_SIZE)
}

fn set_px(rgba: &mut [u8], x: i32, y: i32, color: [u8; 4]) {
    if x < 0 || y < 0 || x >= TRAY_ICON_SIZE as i32 || y >= TRAY_ICON_SIZE as i32 {
        return;
    }
    let idx = ((y as u32 * TRAY_ICON_SIZE + x as u32) * 4) as usize;
    rgba[idx] = color[0];
    rgba[idx + 1] = color[1];
    rgba[idx + 2] = color[2];
    rgba[idx + 3] = color[3];
}

fn draw_rect(rgba: &mut [u8], x0: i32, y0: i32, x1: i32, y1: i32, color: [u8; 4]) {
    for y in y0..=y1 {
        for x in x0..=x1 {
            set_px(rgba, x, y, color);
        }
    }
}

fn draw_line_v(rgba: &mut [u8], x: i32, y0: i32, y1: i32, thickness: i32, color: [u8; 4]) {
    for dx in 0..thickness {
        draw_rect(rgba, x + dx, y0, x + dx, y1, color);
    }
}

fn draw_ring(rgba: &mut [u8], cx: i32, cy: i32, radius: i32, thickness: i32, color: [u8; 4]) {
    let min = -radius - thickness;
    let max = radius + thickness;
    let inner = (radius - thickness).max(0);
    let outer2 = radius * radius;
    let inner2 = inner * inner;
    for dy in min..=max {
        for dx in min..=max {
            let d2 = dx * dx + dy * dy;
            if d2 <= outer2 && d2 >= inner2 {
                set_px(rgba, cx + dx, cy + dy, color);
            }
        }
    }
}

fn draw_base_circle(rgba: &mut [u8]) {
    let center = (TRAY_ICON_SIZE as i32) / 2;
    // Draw slightly beyond the nominal radius so the circle nearly fills the tray slot.
    let radius = center + 1;
    // Blue circle background (matching VS Code info style)
    let fill = [0, 122, 204, 255]; // #007acc
    for y in 0..TRAY_ICON_SIZE as i32 {
        for x in 0..TRAY_ICON_SIZE as i32 {
            let dx = x - center;
            let dy = y - center;
            let d2 = dx * dx + dy * dy;
            let r2 = radius * radius;
            if d2 <= r2 {
                set_px(rgba, x, y, fill);
            }
        }
    }
}

fn draw_j_glyph(rgba: &mut [u8]) {
    let white = [255, 255, 255, 255];
    
    // Larger "J" so the tray glyph stays readable at small sizes.
    draw_rect(rgba, 9, 7, 22, 10, white);
    draw_line_v(rgba, 18, 8, 22, 3, white);
    draw_rect(rgba, 11, 21, 20, 24, white);
    draw_rect(rgba, 8, 19, 11, 23, white);
}

fn draw_coffee_glyph(rgba: &mut [u8]) {
    let white = [20, 24, 30, 255];
    // Slightly larger coffee cup glyph for parity with the "J" icon.
    draw_rect(rgba, 7, 12, 21, 14, white);
    draw_rect(rgba, 7, 21, 21, 23, white);
    draw_line_v(rgba, 7, 12, 23, 2, white);
    draw_line_v(rgba, 20, 12, 23, 2, white);
    draw_ring(rgba, 24, 18, 5, 1, white);
    draw_line_v(rgba, 10, 7, 11, 1, white);
    draw_line_v(rgba, 14, 6, 10, 1, white);
    draw_line_v(rgba, 18, 7, 11, 1, white);
}

pub fn run() {
    let config_store = ConfigStore::new().expect("failed to initialize config store");
    let release_manager = ReleaseManager::new().expect("failed to initialize release manager");
    let runtime_manager = RuntimeManager::new(config_store.paths());
    let manager_service = ManagerService::new(config_store, release_manager, runtime_manager);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState { manager_service })
        .setup(|app| {
            let tray_show = MenuItem::with_id(app, "tray_show_window", "Show", true, None::<&str>)?;
            let tray_stop_all = MenuItem::with_id(
                app,
                "tray_stop_all_services",
                "Stop all services",
                true,
                None::<&str>,
            )?;
            let tray_quit = MenuItem::with_id(app, "tray_quit", "Quit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&tray_show, &tray_stop_all, &tray_quit])?;

            let _tray = TrayIconBuilder::new()
                .icon(build_tray_icon(selected_tray_icon_variant()))
                .menu(&tray_menu)
                .on_menu_event(|tray, event| {
                    let app_handle = tray.app_handle();
                    match event.id().as_ref() {
                        "tray_show_window" => {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "tray_stop_all_services" => {
                            let state = app_handle.state::<AppState>();
                            let _ = state.manager_service.stop_all_runtimes();
                        }
                        "tray_quit" => {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                            emit_quit_prompt_event(app_handle, "tray");
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                if state.manager_service.should_close_to_tray() {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    api.prevent_close();
                    emit_quit_prompt_event(window.app_handle(), "window");
                }
            }
        })
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
            commands::redetect_mcp_client_paths,
            commands::download_or_update_javalens,
            commands::start_runtime,
            commands::stop_runtime,
            commands::get_runtime_status,
            commands::get_services_inventory,
            commands::clean_logs,
            commands::clean_workspaces,
            commands::clean_generated_data,
            commands::probe_services,
            commands::deploy_to_agents,
            commands::get_quit_prompt_context,
            commands::perform_quit_action,
        ])
        .run(tauri::generate_context!())
        .expect("error while running javalens-manager");
}

fn emit_quit_prompt_event(app_handle: &tauri::AppHandle, source: &str) {
    let state = app_handle.state::<AppState>();
    let payload = QuitPromptEvent {
        source: source.to_string(),
        running_services: state.manager_service.running_services_count(),
        tray_enabled: state.manager_service.is_system_tray_enabled(),
    };
    let _ = app_handle.emit("javalens://quit-requested", payload);
}
