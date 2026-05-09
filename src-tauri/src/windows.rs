use crate::{config::AppConfig, reminder_policy::ActiveReminder};
use tauri::{AppHandle, Emitter, Manager, Monitor, PhysicalPosition, PhysicalSize};

pub fn apply_reminder(app: &AppHandle, reminder: ActiveReminder, config: &AppConfig) {
    match reminder {
        ActiveReminder::None => {
            hide_window(app, "banner");
            hide_window(app, "overlay");
        }
        ActiveReminder::Banner => {
            hide_window(app, "overlay");
            show_banner_on_active_monitor(app);
            emit_config(app, "banner", config);
        }
        ActiveReminder::Overlay => {
            hide_window(app, "banner");
            show_overlay_across_all_monitors(app);
            emit_config(app, "overlay", config);
        }
    }
}

pub fn show_banner(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    show_banner_on_active_monitor(app);
    emit_config(app, "banner", config);
    Ok(())
}

pub fn show_overlay(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    hide_window(app, "banner");
    show_overlay_across_all_monitors(app);
    emit_config(app, "overlay", config);
    Ok(())
}

pub fn close_overlay(app: &AppHandle) -> Result<(), String> {
    hide_window(app, "overlay");
    Ok(())
}

fn show_banner_on_active_monitor(app: &AppHandle) {
    let Some(window) = app.get_webview_window("banner") else {
        return;
    };

    let monitor = active_monitor(app).unwrap_or_else(|| primary_monitor(app));

    let pos = monitor.position();
    let size = monitor.size();
    let _ = window.set_position(PhysicalPosition::new(pos.x, pos.y));
    let _ = window.set_size(PhysicalSize::new(size.width, 92));
    let _ = window.set_ignore_cursor_events(true);
    let _ = window.show();
}

fn show_overlay_across_all_monitors(app: &AppHandle) {
    let Some(window) = app.get_webview_window("overlay") else {
        return;
    };

    let monitors = window.available_monitors().unwrap_or_default();
    if monitors.is_empty() {
        let _ = window.show();
        return;
    }

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = 0i32;
    let mut max_y = 0i32;

    for m in &monitors {
        let p = m.position();
        let s = m.size();
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x + s.width as i32);
        max_y = max_y.max(p.y + s.height as i32);
    }

    let width = (max_x - min_x).max(1) as u32;
    let height = (max_y - min_y).max(1) as u32;

    let _ = window.set_position(PhysicalPosition::new(min_x, min_y));
    let _ = window.set_size(PhysicalSize::new(width, height));
    let _ = window.set_ignore_cursor_events(true);
    let _ = window.show();
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

fn active_monitor(app: &AppHandle) -> Option<Monitor> {
    // Find the monitor that contains the mouse cursor
    let pos = cursor_position()?;
    let Some(banner) = app.get_webview_window("banner") else {
        return None;
    };
    let monitors = banner.available_monitors().unwrap_or_default();
    monitors.into_iter().find(|m| {
        let p = m.position();
        let s = m.size();
        pos.0 >= p.x
            && pos.1 >= p.y
            && pos.0 < p.x + s.width as i32
            && pos.1 < p.y + s.height as i32
    })
}

fn primary_monitor(app: &AppHandle) -> Monitor {
    app.get_webview_window("main")
        .and_then(|w| w.primary_monitor().ok())
        .flatten()
        .unwrap_or_else(|| {
            // This should never happen. Fallback: same as active monitor logic without cursor.
            app.get_webview_window("banner")
                .and_then(|w| w.available_monitors().ok())
                .and_then(|mut m| m.pop())
                .expect("no monitor available")
        })
}

#[cfg(windows)]
fn cursor_position() -> Option<(i32, i32)> {
    unsafe {
        let mut pt: windows::Win32::Foundation::POINT = std::mem::zeroed();
        if windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt).is_ok() {
            Some((pt.x, pt.y))
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
fn cursor_position() -> Option<(i32, i32)> {
    None
}
