pub mod driver;
pub mod effects;
pub mod tray;

use std::sync::atomic::Ordering;
use std::sync::Arc;

use effects::battery::BatteryState;
use effects::{EffectManager, EffectMode};
use tauri::{Manager, State};

pub struct AppState {
    pub manager: Arc<EffectManager>,
}

impl AppState {
    fn set_power(&self, on: bool) {
        self.manager.params().power.store(on, Ordering::Relaxed);
        self.manager.apply();
    }

    fn toggle_power(&self) {
        let on = !self.manager.params().power.load(Ordering::Relaxed);
        self.set_power(on);
    }

    fn set_mode(&self, mode: EffectMode) {
        *self.manager.params().mode.lock().unwrap() = mode;
        self.manager.apply();
    }

    fn shutdown(&self) {
        self.manager.shutdown(true);
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
fn set_power(state: State<'_, AppState>, on: bool) -> Result<(), String> {
    state.set_power(on);
    Ok(())
}

#[tauri::command]
fn set_mode(state: State<'_, AppState>, mode: String) -> Result<(), String> {
    let mode = EffectMode::from_name(&mode).ok_or_else(|| format!("unknown mode: {mode}"))?;
    state.set_mode(mode);
    Ok(())
}

#[tauri::command]
fn set_speed(state: State<'_, AppState>, speed: u8) -> Result<(), String> {
    state.manager.params().speed.store(speed.clamp(1, 10), Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
fn set_sensitivity(state: State<'_, AppState>, sensitivity: u32) -> Result<(), String> {
    state
        .manager
        .params()
        .sensitivity
        .store(sensitivity.clamp(0, 100), Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
fn set_battery_threshold(state: State<'_, AppState>, threshold: u32) -> Result<(), String> {
    state
        .manager
        .params()
        .battery_threshold
        .store(threshold.clamp(1, 100), Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
fn set_static_level(state: State<'_, AppState>, level: u8) -> Result<(), String> {
    state
        .manager
        .params()
        .static_level
        .store(level.clamp(0, 2), Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
fn set_tray_close(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state.manager.params().tray_close.store(enabled, Ordering::Relaxed);
    Ok(())
}

#[derive(serde::Serialize)]
struct StatusInfo {
    power: bool,
    mode: String,
    static_level: u8,
    interval_ms: u64,
    speed: u8,
    sensitivity: u32,
    driver_supported: bool,
    driver_path: String,
    driver_error: Option<String>,
    audio_error: Option<String>,
    battery: Option<BatteryState>,
    autostart: bool,
    tray_close: bool,
    os: String,
}

#[tauri::command]
fn get_status(state: State<'_, AppState>) -> StatusInfo {
    let p = state.manager.params();
    let power = p.power.load(Ordering::Relaxed);
    let mode = p.mode.lock().unwrap().name().to_string();
    let speed = p.speed.load(Ordering::Relaxed);
    let sensitivity = p.sensitivity.load(Ordering::Relaxed);
    let static_level = p.static_level.load(Ordering::Relaxed);
    let tray_close = p.tray_close.load(Ordering::Relaxed);
    let interval_ms = effects::interval_from_speed(speed);
    let (driver_supported, driver_path) = state.manager.driver_info();
    let driver_error = state.manager.last_error();
    let audio_error = if mode == "audio" {
        driver_error.clone()
    } else {
        None
    };

    let threshold = p.battery_threshold.load(Ordering::Relaxed) as u32;
    let battery = effects::battery::read_battery().map(|mut b| {
        b.low = b.discharging && b.percent <= threshold;
        b
    });

    StatusInfo {
        power,
        mode,
        static_level,
        interval_ms,
        speed,
        sensitivity,
        driver_supported,
        driver_path,
        driver_error,
        audio_error,
        battery,
        autostart: autostart_enabled(),
        tray_close,
        os: std::env::consts::OS.to_string(),
    }
}

#[derive(serde::Serialize)]
struct UdevResult {
    ok: bool,
    message: String,
}

#[tauri::command]
fn install_udev() -> UdevResult {
    #[cfg(target_os = "linux")]
    {
        driver::linux::install_udev_rule().map_or_else(
            |e| UdevResult { ok: false, message: e },
            |m| UdevResult { ok: true, message: m },
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        UdevResult {
            ok: false,
            message: "udev permissions are Linux-only".into(),
        }
    }
}

#[tauri::command]
fn set_autostart(enabled: bool) -> Result<(), String> {
    set_autostart_impl(enabled)
}

#[tauri::command]
fn get_autostart() -> bool {
    autostart_enabled()
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

// ---------------------------------------------------------------------------
// Autostart management
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn autostart_file() -> std::path::PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from(
            std::env::var("HOME").unwrap_or_else(|_| ".".into()),
        ).join(".config"));
    base.join("autostart").join("glowfx.desktop")
}

#[cfg(target_os = "linux")]
fn autostart_enabled() -> bool {
    autostart_file().exists()
}

#[cfg(target_os = "linux")]
fn set_autostart_impl(enabled: bool) -> Result<(), String> {
    let path = autostart_file();
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let content = format!(
            "[Desktop Entry]\nType=Application\nName=GlowFX\nComment=Lenovo keyboard backlight control\nExec=\"{}\"\nIcon=glowfx\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
            exe.display()
        );
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::write(&path, content).map_err(|e| e.to_string())
    } else {
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(target_os = "windows")]
fn autostart_enabled() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;
    let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(
        r"Software\Microsoft\Windows\CurrentVersion\Run",
    ) else {
        return false;
    };
    hkcu.get_value::<String, _>("GlowFX").is_ok()
}

#[cfg(target_os = "windows")]
fn set_autostart_impl(enabled: bool) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run")
        .map_err(|e| e.to_string())?;
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        key.set_value("GlowFX", &exe.display().to_string())
            .map_err(|e| e.to_string())
    } else {
        let _ = key.delete_value("GlowFX");
        Ok(())
    }
}

#[cfg(all(not(target_os = "linux"), not(target_os = "windows")))]
fn autostart_enabled() -> bool {
    false
}

#[cfg(all(not(target_os = "linux"), not(target_os = "windows")))]
fn set_autostart_impl(_enabled: bool) -> Result<(), String> {
    Err("Autostart is not supported on this platform yet".into())
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let driver = driver::detect_driver();
    let manager = effects::EffectManager::new(driver);

    // Graceful SIGINT/SIGTERM: stop loops, restore backlight, exit.
    let restore = Arc::clone(&manager);
    let _ = ctrlc::set_handler(move || {
        restore.shutdown(true);
        std::process::exit(0);
    });

    let state = AppState { manager };

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            set_power,
            set_mode,
            set_speed,
            set_sensitivity,
            set_battery_threshold,
            set_static_level,
            set_tray_close,
            get_status,
            install_udev,
            set_autostart,
            get_autostart,
            quit_app,
        ])
        .setup(|app| {
            tray::create_tray(app.handle())?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building GlowFX")
        .run(|app_handle, event| {
            // Restore backlight on app exit (incl. tray Quit).
            if let tauri::RunEvent::Exit = event {
                let st = app_handle.state::<AppState>();
                st.shutdown();
            }
        });
}