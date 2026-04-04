use tauri::{AppHandle, Emitter, State};
use tauri_plugin_autostart::ManagerExt;

use crate::app_state::AppState;
use crate::models::{
    DashboardState, ProviderKind, SaveProviderSettingsInput, SaveRuntimePreferencesInput,
    SettingsState,
};

#[tauri::command]
pub fn get_dashboard_state(state: State<'_, AppState>) -> Result<DashboardState, String> {
    state.build_dashboard_state().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_settings_state(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<SettingsState, String> {
    state
        .load_settings_state(&app)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn refresh_now(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DashboardState, String> {
    let dashboard = state.refresh_all().await.map_err(|error| error.to_string())?;
    app.emit("dashboard://updated", dashboard.clone())
        .map_err(|error: tauri::Error| error.to_string())?;
    Ok(dashboard)
}

#[tauri::command]
pub fn save_provider_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    input: SaveProviderSettingsInput,
) -> Result<SettingsState, String> {
    state
        .save_provider_settings(input)
        .map_err(|error| error.to_string())?;
    state
        .load_settings_state(&app)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_runtime_preferences(
    app: AppHandle,
    state: State<'_, AppState>,
    input: SaveRuntimePreferencesInput,
) -> Result<SettingsState, String> {
    state
        .save_runtime_preferences(input.refresh_interval_secs)
        .map_err(|error| error.to_string())?;
    state
        .load_settings_state(&app)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_launch_at_login(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|error| error.to_string())?;
    } else {
        manager.disable().map_err(|error| error.to_string())?;
    }

    manager.is_enabled().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn toggle_provider(
    _app: AppHandle,
    state: State<'_, AppState>,
    provider: ProviderKind,
) -> Result<DashboardState, String> {
    state
        .toggle_provider(provider)
        .map_err(|error| error.to_string())?;
    state.build_dashboard_state().map_err(|error| error.to_string())
}
