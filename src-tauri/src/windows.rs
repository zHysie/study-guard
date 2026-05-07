use crate::{config::AppConfig, reminder_policy::ActiveReminder};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};

pub fn apply_reminder(app: &AppHandle, reminder: ActiveReminder, config: &AppConfig) {
    match reminder {
        ActiveReminder::None => {
            hide_window(app, "banner");
            hide_window(app, "overlay");
        }
        ActiveReminder::Banner => {
            hide_window(app, "overlay");
            show_window(app, "banner");
            emit_config(app, "banner", config);
        }
        ActiveReminder::Overlay => {
            hide_window(app, "banner");
            show_window(app, "overlay");
            emit_config(app, "overlay", config);
        }
    }
}

pub fn show_banner(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    show_window(app, "banner");
    emit_config(app, "banner", config);
    Ok(())
}

pub fn show_overlay(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    hide_window(app, "banner");
    show_window(app, "overlay");
    emit_config(app, "overlay", config);
    Ok(())
}

pub fn close_overlay(app: &AppHandle) -> Result<(), String> {
    hide_window(app, "overlay");
    Ok(())
}

fn show_window(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        if label == "banner" {
            if let Ok(Some(monitor)) = window.current_monitor() {
                let monitor_position = monitor.position();
                let monitor_size = monitor.size();
                let _ = window.set_position(PhysicalPosition::new(
                    monitor_position.x,
                    monitor_position.y,
                ));
                let _ = window.set_size(PhysicalSize::new(monitor_size.width, 72));
            }
            let _ = window.set_ignore_cursor_events(true);
        }
        let _ = window.show();
        if label == "overlay" {
            let _ = window.set_ignore_cursor_events(true);
        }
    }
}

fn hide_window(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.hide();
    }
}

fn emit_config(app: &AppHandle, label: &str, config: &AppConfig) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.emit("reminder-config", config);
    }
}
