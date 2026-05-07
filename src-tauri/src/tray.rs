use crate::{monitor, monitor::AppState, windows};
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{Menu, MenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

static ICON_ACTIVE_BYTES: &[u8] = include_bytes!("../icons/icon_active.png");
static ICON_PAUSED_BYTES: &[u8] = include_bytes!("../icons/icon_paused.png");

fn decode_png_to_image(bytes: &[u8]) -> Image<'static> {
    let img = image::load_from_memory(bytes)
        .expect("failed to decode tray icon PNG")
        .into_rgba8();
    let (width, height) = img.dimensions();
    Image::new_owned(img.into_raw(), width, height)
}

fn tray_icon(paused: bool) -> Image<'static> {
    let bytes = if paused { ICON_PAUSED_BYTES } else { ICON_ACTIVE_BYTES };
    decode_png_to_image(bytes)
}

fn build_tray_menu(app: &AppHandle, paused: bool) -> tauri::Result<Menu<tauri::Wry>> {
    MenuBuilder::new(app)
        .text("open_settings", "打开")
        .text("toggle_pause", if paused { "恢复监控" } else { "暂停监控" })
        .separator()
        .text("quit", "退出")
        .build()
}

fn update_tray(app: &AppHandle, paused: bool) {
    if let Some(tray) = app.tray_by_id("study-guardian") {
        let _ = tray.set_icon(Some(tray_icon(paused)));
        if let Ok(menu) = build_tray_menu(app, paused) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

pub fn setup_tray(app: &tauri::AppHandle, state: Arc<AppState>) -> tauri::Result<()> {
    let initial_paused = *state.paused.lock().expect("paused lock poisoned");
    let menu = build_tray_menu(app, initial_paused)?;

    let tray_state = state.clone();
    TrayIconBuilder::with_id("study-guardian")
        .icon(tray_icon(initial_paused))
        .tooltip("学习守门员")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "open_settings" => show_main_window(app),
            "toggle_pause" => {
                let paused = {
                    if let Ok(mut p) = tray_state.paused.lock() {
                        *p = !*p;
                        *p
                    } else {
                        false
                    }
                };
                update_tray(app, paused);
                monitor::evaluate_once(app, &tray_state);
            }
            "quit" => {
                windows::apply_reminder(
                    app,
                    crate::reminder_policy::ActiveReminder::None,
                    &tray_state
                        .config
                        .lock()
                        .map(|config| config.clone())
                        .unwrap_or_default(),
                );
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
