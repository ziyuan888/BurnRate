use anyhow::Result;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::app_state::AppState;
use crate::models::DashboardState;

pub fn configure(app: &AppHandle, state: &AppState) -> Result<()> {
    let refresh_item = MenuItemBuilder::with_id("refresh", "立即刷新").build(app)?;
    let settings_item = MenuItemBuilder::with_id("settings", "打开设置").build(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", "退出").build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[&refresh_item, &settings_item, &quit_item])
        .build()?;

    let icon = app
        .default_window_icon()
        .cloned()
        .expect("default icon should exist");

    let tooltip = state.build_tooltip_text();

    let tray_state = state.clone();
    let app_handle = app.clone();

    TrayIconBuilder::with_id("burn-rate")
        .icon(icon)
        .icon_as_template(true)
        .tooltip(&tooltip)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app: &AppHandle, event| match event.id.as_ref() {
            "refresh" => {
                let state = tray_state.clone();
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Ok(dashboard) = state.refresh_all().await {
                        update_tray_tooltip(&app_handle, &state);
                        let _ = app_handle.emit("dashboard://updated", dashboard);
                    }
                });
            }
            "settings" => {
                let _ = show_settings(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(move |tray, event| {
            tauri_plugin_positioner::on_tray_event::<tauri::Wry>(tray.app_handle(), &event);
            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    let _ = toggle_popover(tray.app_handle());
                }
                TrayIconEvent::DoubleClick { .. } => {
                    let _ = show_settings(tray.app_handle());
                }
                _ => {}
            }
        })
        .build(app)?;

    if let Some(popover) = app_handle.get_webview_window("popover") {
        let popover_clone = popover.clone();
        popover.on_window_event(move |event| {
            if let tauri::WindowEvent::Focused(false) = event {
                let _ = popover_clone.hide();
            }
        });
    }

    Ok(())
}

pub fn update_tray_tooltip(app: &AppHandle, state: &AppState) {
    let tooltip = state.build_tooltip_text();
    if let Some(tray) = app.tray_by_id("burn-rate") {
        let _ = tray.set_tooltip(Some(&tooltip));
    }
}

pub fn update_tray_icon(app: &AppHandle, dashboard: &DashboardState) {
    // Compute the most "stressed" ratios across all enabled providers.
    let mut primary_ratio = 0.0_f64;
    let mut secondary_ratio = 0.0_f64;

    for provider in &dashboard.providers {
        if !provider.is_enabled {
            continue;
        }
        // numeric_value for Zhipu/Minimax/Kimi-coding is a usage ratio 0..1.
        // For Kimi balance mode it is a currency amount — skip when > 1.0.
        if let Some(pct) = provider.headline_value.strip_suffix('%') {
            if let Ok(val) = pct.parse::<f64>() {
                let ratio = val / 100.0;
                if ratio <= 1.0 {
                    primary_ratio = primary_ratio.max(ratio);
                }
            }
        }
        if let Some(sec) = provider.secondary_percent {
            if sec >= 0.0 && sec <= 1.0 {
                secondary_ratio = secondary_ratio.max(sec);
            }
        }
    }

    let rgba = crate::tray_icon::generate_meter_icon(primary_ratio, secondary_ratio);
    let image = tauri::image::Image::new_owned(rgba, 22, 22);
    if let Some(tray) = app.tray_by_id("burn-rate") {
        let _ = tray.set_icon(Some(image));
        let _ = tray.set_icon_as_template(true);
    }
}

fn toggle_popover(app: &AppHandle) -> Result<()> {
    let popover = app
        .get_webview_window("popover")
        .expect("popover window should exist");

    if popover.is_visible()? {
        popover.hide()?;
        return Ok(());
    }

    popover.move_window(Position::TrayCenter)?;
    popover.show()?;
    popover.set_focus()?;
    Ok(())
}

fn show_settings(app: &AppHandle) -> Result<()> {
    let settings = app
        .get_webview_window("settings")
        .expect("settings window should exist");
    settings.show()?;
    settings.set_focus()?;
    Ok(())
}
