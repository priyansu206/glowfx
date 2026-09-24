//! Battery Guard effect: poll the battery and flash a low-battery alert.
//! Polling stays slow (every ~1.5 s) so it never contributes bus traffic.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{capped_sleep, is_stopped, EffectShared};

#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct BatteryState {
    pub percent: u32,
    pub discharging: bool,
    pub low: bool,
}

pub fn run(shared: Arc<EffectShared>, stop: Arc<AtomicBool>) {
    let mut last = Instant::now();
    let mut flash_on = false;
    let mut alert_armed = false;

    while !is_stopped(&stop) {
        if last.elapsed() >= Duration::from_millis(1500) {
            if let Some(b) = read_battery() {
                let threshold = shared.params.battery_threshold.load(Ordering::Relaxed) as u32;
                let low = b.discharging && b.percent <= threshold;
                alert_armed = low;
            } else {
                alert_armed = false;
            }
            last = Instant::now();
        }

        if alert_armed {
            flash_on = !flash_on;
            shared.set_level(if flash_on { 2 } else { 0 });
            capped_sleep(900);
        } else {
            shared.set_level(1); // guide-level brightness while idle
            capped_sleep(1500);
        }
    }
}

/// Read battery percentage + discharging state from the OS.
pub fn read_battery() -> Option<BatteryState> {
    #[cfg(target_os = "linux")]
    {
        read_linux()
    }
    #[cfg(target_os = "windows")]
    {
        read_windows()
    }
    #[cfg(all(not(target_os = "linux"), not(target_os = "windows")))]
    {
        None
    }
}

#[cfg(target_os = "linux")]
fn read_linux() -> Option<BatteryState> {
    use std::fs;
    use std::path::Path;

    let base = Path::new("/sys/class/power_supply");
    let dirs = fs::read_dir(base).ok()?;
    for entry in dirs.flatten() {
        let dir = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("BAT") {
            continue;
        }
        let capacity: u32 = fs::read_to_string(dir.join("capacity"))
            .ok()?
            .trim()
            .parse()
            .ok()?;
        let status = fs::read_to_string(dir.join("status"))
            .ok()
            .unwrap_or_default();
        let discharging = status.trim() == "Discharging";
        return Some(BatteryState {
            percent: capacity.min(100),
            discharging,
            low: false,
        });
    }
    None
}

#[cfg(target_os = "windows")]
fn read_windows() -> Option<BatteryState> {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Win32_Battery {
        EstimatedChargeRemaining: Option<u32>,
        BatteryStatus: Option<i32>,
    }

    let conn = wmi::WMIConnection::new().ok()?;
    let b: Result<Win32_Battery, _> = conn.get();
    let b = b.ok()?;
    let percent = b.EstimatedChargeRemaining.unwrap_or(0).min(100);
    let discharging = b.BatteryStatus == Some(1); // 1 = discharging, 2 = on AC
    Some(BatteryState {
        percent,
        discharging,
        low: false,
    })
}