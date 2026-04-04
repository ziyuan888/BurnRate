mod app_state;
mod commands;
mod models;
pub mod providers;
pub mod storage;
mod tray;

use app_state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ))
        .plugin(tauri_plugin_positioner::init())
        .setup(|app| {
            let state = AppState::new(app.handle())?;
            state.spawn_background_refresh(app.handle().clone());
            tray::configure(app.handle(), &state)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard_state,
            commands::get_settings_state,
            commands::refresh_now,
            commands::save_provider_settings,
            commands::save_runtime_preferences,
            commands::set_launch_at_login,
            commands::quit_app,
            commands::toggle_provider,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
