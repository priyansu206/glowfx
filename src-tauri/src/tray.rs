//! Cross-platform system tray with quick power / mode / quit controls.

use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use crate::effects::EffectMode;

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let Some(icon) = app.default_window_icon().cloned().or_else(|| {
        Image::from_bytes(include_bytes!("../icons/icon.png")).ok()
    }) else {
        return Ok(());
    };

    let power = MenuItem::with_id(app, "power", "Power", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let effects = Submenu::with_items(
        app,
        "Effects",
        true,
        &[
            &MenuItem::with_id(app, "mode_static", "Static", true, None::<&str>)?,
            &MenuItem::with_id(app, "mode_breathing", "Breathing", true, None::<&str>)?,
            &MenuItem::with_id(app, "mode_strobing", "Strobing", true, None::<&str>)?,
            &MenuItem::with_id(app, "mode_audio", "Audio Visualizer", true, None::<&str>)?,
            &MenuItem::with_id(app, "mode_battery", "Battery Guard", true, None::<&str>)?,
            &MenuItem::with_id(app, "mode_storm", "Storm", true, None::<&str>)?,
            &MenuItem::with_id(app, "mode_ripple", "Ripple", true, None::<&str>)?,
            &MenuItem::with_id(app, "mode_morse", "Morse Code", true, None::<&str>)?,
        ],
    )?;
    let show = MenuItem::with_id(app, "show", "Show GlowFX", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu =
        Menu::with_items(app, &[&power, &sep, &effects, &show, &sep2, &quit])?;

    TrayIconBuilder::with_id("glowfx-tray")
        .icon(icon)
        .tooltip("GlowFX — Lenovo backlight")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app_handle, event| {
            match event.id.as_ref() {
                "power" => {
                    let st = app_handle.state::<crate::AppState>();
                    st.toggle_power();
                }
                "mode_static" => st_set_mode(app_handle, "static"),
                "mode_breathing" => st_set_mode(app_handle, "breathing"),
                "mode_strobing" => st_set_mode(app_handle, "strobing"),
                "mode_audio" => st_set_mode(app_handle, "audio"),
                "mode_battery" => st_set_mode(app_handle, "battery"),
                "mode_storm" => st_set_mode(app_handle, "storm"),
                "mode_ripple" => st_set_mode(app_handle, "ripple"),
                "mode_morse" => st_set_mode(app_handle, "morse"),
                "show" => {
                    if let Some(win) = app_handle.get_webview_window("main") {
                        let _ = win.show();
                        let _ = win.set_focus();
                    }
                }
                "quit" => app_handle.exit(0),
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

fn st_set_mode(app_handle: &AppHandle, name: &str) {
    if let Some(mode) = EffectMode::from_name(name) {
        let st = app_handle.state::<crate::AppState>();
        st.set_mode(mode);
    }
}