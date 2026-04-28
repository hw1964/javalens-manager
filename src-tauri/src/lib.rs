mod commands;
mod config;
mod manager_service;
mod release_manager;
mod runtime_manager;

use config::ConfigStore;
use manager_service::ManagerService;
use release_manager::ReleaseManager;
use runtime_manager::{RuntimeManager, RuntimePhase};
use serde::Serialize;
use tauri::{
    image::Image,
    menu::{Menu, MenuBuilder},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, Runtime, WindowEvent,
};

pub struct AppState {
    pub manager_service: ManagerService,
}

const TRAY_ICON_SIZE: u32 = 32;

/// Sprint 12 (v0.12.0): size of the per-workspace status icons rendered
/// inside the tray menu.
const STATUS_ICON_SIZE: u32 = 16;

/// Stable id of the singleton tray icon. The tray is built once in setup
/// and looked up via `app_handle.tray_by_id(TRAY_ICON_ID)` whenever the
/// menu needs rebuilding.
const TRAY_ICON_ID: &str = "javalens-tray";

/// Sprint 12: how often to refresh the tray menu in the background, so
/// external state changes (e.g. a workspace's javalens process getting
/// killed from the shell) propagate to the per-workspace status icons.
/// Aligns with the runtime-manager's health-check cadence.
const TRAY_REFRESH_INTERVAL_SECS: u64 = 5;

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
    // Dark blue circle background (#1c3a74)
    let fill = [28, 58, 116, 255]; // #1c3a74
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

/// Sprint 12 (v0.12.0): paint a 16×16 RGBA filled circle in the colour
/// matching `phase`, transparent background. Used as the per-workspace
/// status indicator inside `IconMenuItem`s in the tray menu.
///
/// Same procedural-drawing approach as `build_tray_icon` (no static asset
/// files committed). Tauri scales the 16×16 source for HiDPI displays.
fn build_status_icon(phase: &RuntimePhase) -> Image<'static> {
    // Tailwind-ish saturated colours that read well on both light and dark
    // panel backgrounds across GNOME / KDE / macOS / Win32.
    let color: [u8; 4] = match phase {
        RuntimePhase::Running => [34, 197, 94, 255],   // #22C55E green
        RuntimePhase::Starting => [245, 158, 11, 255], // #F59E0B amber
        RuntimePhase::Failed => [239, 68, 68, 255],    // #EF4444 red
        RuntimePhase::Stopped => [107, 114, 128, 255], // #6B7280 slate gray
    };
    let size = STATUS_ICON_SIZE as i32;
    let center = size / 2;
    // One-pixel inset so the circle doesn't touch the canvas edge — looks
    // sharper at native size and scales cleanly when the OS upsamples.
    let radius = (size / 2) - 1;
    let r2 = radius * radius;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let dx = x - center;
            let dy = y - center;
            if dx * dx + dy * dy <= r2 {
                let idx = ((y * size + x) * 4) as usize;
                rgba[idx] = color[0];
                rgba[idx + 1] = color[1];
                rgba[idx + 2] = color[2];
                rgba[idx + 3] = color[3];
            }
        }
    }
    Image::new_owned(rgba, STATUS_ICON_SIZE, STATUS_ICON_SIZE)
}

/// Sprint 12 (v0.12.0): build the tray menu reflecting the current set of
/// workspaces and their aggregated phases.
///
/// Menu shape:
///   Show
///   ─────
///   <workspace-1-icon> <workspace-1-name>
///   <workspace-2-icon> <workspace-2-name>
///   …
///   ─────                            (only if at least one workspace)
///   Start all services
///   Stop all services
///   ─────
///   Quit
///
/// Per-workspace entries use `IconMenuItem` with id
/// `tray_workspace_toggle:<workspace_name>`; the click handler parses the
/// suffix to identify which workspace to start or stop.
fn rebuild_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let summaries = app
        .state::<AppState>()
        .manager_service
        .workspace_status_summary();

    let mut builder = MenuBuilder::new(app)
        .text("tray_show_window", "Show")
        .separator();

    for summary in &summaries {
        let icon = build_status_icon(&summary.phase);
        let id = format!("tray_workspace_toggle:{}", summary.workspace_name);
        builder = builder.icon(id, &summary.workspace_name, icon);
    }

    if !summaries.is_empty() {
        builder = builder.separator();
    }

    builder
        .text("tray_start_all_services", "Start all services")
        .text("tray_stop_all_services", "Stop all services")
        .separator()
        .text("tray_quit", "Quit")
        .build()
}

/// Sprint 12 (v0.12.0): rebuild the tray menu and swap it onto the live
/// tray icon. Cheap (≤10 menu items typically) — sub-50 ms on Win32 /
/// macOS / GNOME-with-AppIndicator. Errors are logged but not propagated:
/// a stale menu is preferable to a panic from the periodic refresh task.
fn refresh_tray_menu<R: Runtime>(app: &AppHandle<R>) {
    if !app
        .state::<AppState>()
        .manager_service
        .is_system_tray_enabled()
    {
        return;
    }
    let menu = match rebuild_tray_menu(app) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("javalens-manager: rebuild_tray_menu failed: {e}");
            return;
        }
    };
    if let Some(tray) = app.tray_by_id(TRAY_ICON_ID) {
        if let Err(e) = tray.set_menu(Some(menu)) {
            eprintln!("javalens-manager: tray.set_menu failed: {e}");
        }
    }
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
            // Sprint 12 (v0.12.0): the menu is now built dynamically from
            // workspace_status_summary() so per-workspace toggle entries with
            // status-coloured icons can land in the tray. The TrayIconBuilder
            // gets a stable id so the periodic refresh task and the click
            // handlers can find it.
            let initial_menu = rebuild_tray_menu(app.handle())?;

            let _tray = TrayIconBuilder::with_id(TRAY_ICON_ID)
                .icon(build_tray_icon(selected_tray_icon_variant()))
                .menu(&initial_menu)
                .on_menu_event(|tray, event| {
                    let app_handle = tray.app_handle();
                    let id = event.id().as_ref();
                    match id {
                        "tray_show_window" => {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "tray_start_all_services" => {
                            let state = app_handle.state::<AppState>();
                            let _ = state.manager_service.start_all_runtimes();
                            refresh_tray_menu(app_handle);
                        }
                        "tray_stop_all_services" => {
                            let state = app_handle.state::<AppState>();
                            let _ = state.manager_service.stop_all_runtimes();
                            refresh_tray_menu(app_handle);
                        }
                        "tray_quit" => {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                            emit_quit_prompt_event(app_handle, "tray");
                        }
                        other => {
                            if let Some(workspace_name) =
                                other.strip_prefix("tray_workspace_toggle:")
                            {
                                let state = app_handle.state::<AppState>();
                                let _ = state.manager_service.toggle_workspace(workspace_name);
                                refresh_tray_menu(app_handle);
                            }
                        }
                    }
                })
                .build(app)?;

            // Sprint 12: periodic refresh so external state changes (e.g. a
            // workspace's javalens process getting killed from the shell)
            // propagate to the per-workspace status icons. Cheap: a Menu
            // rebuild + tray.set_menu swap (Tauri dispatches set_menu to the
            // main thread internally), every TRAY_REFRESH_INTERVAL_SECS.
            let refresh_handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(
                    TRAY_REFRESH_INTERVAL_SECS,
                ));
                refresh_tray_menu(&refresh_handle);
            });

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
            commands::set_project_workspace,
            commands::rename_workspace,
            commands::delete_workspace,
            commands::rename_project,
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
