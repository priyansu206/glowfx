//! Battery Guard effect: polls the battery and (a) flashes SOS at or below the
//! critical threshold while discharging, (b) pulses on charging-complete, and
//! (c) otherwise holds a dim guide level (dimmed further by auto-dim).
//! Polling stays slow (every ~1.5 s) so it never contributes bus traffic.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{capped_sleep, is_stopped, EffectShared};

const POLL_MS: u64 = 1500;
const PATTERN_MS: u64 = 200;

/// SOS: S S S O O O S S S then pause. S = 2 on + 2 off ticks, O = 3 on + 1 off.
fn sos_pattern() -> Vec<u8> {
    let mut v = Vec::new();
    for _ in 0..3 {
        v.extend_from_slice(&[2, 0]); // S
    }
    for _ in 0..3 {
        v.extend_from_slice(&[2, 2, 2, 0]); // O
    }
    for _ in 0..3 {
        v.extend_from_slice(&[2, 0]); // S
    }
    v.extend_from_slice(&[0, 0, 0, 0]); // pause
    v
}

/// Charging-complete: three quick blinks, a settle, then hold steady.
fn charged_pattern() -> Vec<u8> {
    vec![2, 0, 2, 0, 2, 0, 2, 2, 2, 0, 0, 0, 0]
}

#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct BatteryState {
    pub percent: u32,
    pub discharging: bool,
    pub low: bool,
}

enum Mode {
    Guard,
    Sos,
    Charged,
}

pub fn run(shared: Arc<EffectShared>, stop: Arc<AtomicBool>) {
    let sos = sos_pattern();
    let charged = charged_pattern();

    let mut last_poll = Instant::now();
    let mut mode = Mode::Guard;
    let mut pattern_step = 0usize;
    let mut pattern_tick = Instant::now();

    while !is_stopped(&stop) {
        // Refresh battery snapshot every 1.5 s.
        if last_poll.elapsed() >= Duration::from_millis(POLL_MS) {
            let threshold = shared.params.battery_threshold.load(Ordering::Relaxed) as u32;
            let critical = shared.params.critical_threshold.load(Ordering::Relaxed) as u32;
            mode = match read_battery() {
                Some(b) if b.discharging && b.percent <= critical.min(threshold) => Mode::Sos,
                Some(b) if !b.discharging && b.percent >= 100 => Mode::Charged,
                _ => Mode::Guard,
            };
            last_poll = Instant::now();
        }

        match mode {
            Mode::Sos => {
                if pattern_tick.elapsed() >= Duration::from_millis(PATTERN_MS) {
                    let level = sos[pattern_step % sos.len()];
                    pattern_step += 1;
                    pattern_tick = Instant::now();
                    shared.emit(level);
                }
                capped_sleep(60);
            }
            Mode::Charged => {
                if pattern_tick.elapsed() >= Duration::from_millis(250) {
                    let level = charged[pattern_step % charged.len()];
                    pattern_step += 1;
                    pattern_tick = Instant::now();
                    shared.emit(level);
                }
                capped_sleep(60);
            }
            Mode::Guard => {
                shared.emit(1); // dim guide level while on battery/AC
                capped_sleep(POLL_MS);
            }
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