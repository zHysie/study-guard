#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use study_guardian::{
    config::{self, AppConfig},
    monitor::{self, AppState, MonitorStatus},
    native_messaging, paths, tray, windows,
};
use tauri::Manager;

#[tauri::command]
fn get_config(state: tauri::State<'_, Arc<AppState>>) -> Result<AppConfig, String> {
    Ok(state.config.lock().map_err(|err| err.to_string())?.clone())
}

#[tauri::command]
fn save_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    config: AppConfig,
) -> Result<AppConfig, String> {
    let sanitized =
        config::save_to_path(&state.config_path, &config).map_err(|err| err.to_string())?;
    *state.config.lock().map_err(|err| err.to_string())? = sanitized.clone();
    monitor::evaluate_once(&app, state.inner());
    Ok(sanitized)
}

#[tauri::command]
fn set_mock_url(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    url: Option<String>,
) -> Result<MonitorStatus, String> {
    state.url_provider.set_url(url);
    Ok(monitor::evaluate_once(&app, state.inner()))
}

#[tauri::command]
fn get_status(state: tauri::State<'_, Arc<AppState>>) -> Result<MonitorStatus, String> {
    Ok(state.status.lock().map_err(|err| err.to_string())?.clone())
}

#[tauri::command]
fn test_banner(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let config = state.config.lock().map_err(|err| err.to_string())?.clone();
    windows::show_banner(&app, &config)
}

#[tauri::command]
fn test_overlay(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let config = state.config.lock().map_err(|err| err.to_string())?.clone();
    windows::show_overlay(&app, &config)
}

#[tauri::command]
fn close_overlay_for_test(app: tauri::AppHandle) -> Result<(), String> {
    windows::close_overlay(&app)
}

#[tauri::command]
fn set_paused(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    paused: bool,
) -> Result<MonitorStatus, String> {
    *state.paused.lock().map_err(|err| err.to_string())? = paused;
    Ok(monitor::evaluate_once(&app, state.inner()))
}

fn main() {
    if std::env::args().any(|arg| arg == "--native-messaging-host") {
        if let Err(err) = native_messaging::run_stdio_host() {
            eprintln!("native messaging host failed: {err}");
        }
        return;
    }

    let config_dir = paths::fallback_config_dir();
    let config_path = config_dir.join("config.json");
    let config = config::load_or_default(&config_path).expect("failed to load app config");
    let native_url_path = monitor::default_native_url_path(&config_path);
    let state = Arc::new(AppState::new(config_path, config, native_url_path));
    let setup_state = state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .manage(state)
        .setup(move |app| {
            study_guardian::native_host::ensure_registered(&config_dir);
            if let Some(window) = app.get_webview_window("main") {
                let w = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = &event {
                        api.prevent_close();
                        let _ = w.hide();
                    }
                });
            }
            tray::setup_tray(app.handle(), setup_state.clone())?;
            monitor::start_monitor(app.handle().clone(), setup_state.clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            set_mock_url,
            get_status,
            test_banner,
            test_overlay,
            close_overlay_for_test,
            set_paused
        ])
        .run(tauri::generate_context!())
        .expect("error while running 学习守门员");
}
