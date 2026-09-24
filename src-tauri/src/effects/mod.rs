//! Effect engine. Runs exactly one loop at a time; every driver write is
//! quantized to {0,1,2} and rate-limited to ≥100 ms (≤10 Hz) to protect the
//! LPC/ACPI bus from flooding (input lag / fan stutter protection).

pub mod audio;
pub mod battery;
pub mod breathing;
pub mod strobing;

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::thread::{self, JoinHandle};

use crate::driver::BacklightDriver;

pub type DriverMutex = Arc<StdMutex<Box<dyn BacklightDriver>>>;

/// The three discrete states the Lenovo EC understands.
pub const BACKLIGHT_OFF: u8 = 0;
pub const BACKLIGHT_LOW: u8 = 1;
pub const BACKLIGHT_HIGH: u8 = 2;

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize)]
pub enum EffectMode {
    Static,
    Breathing,
    Strobing,
    Audio,
    Battery,
    Off,
}

impl EffectMode {
    pub fn name(&self) -> &'static str {
        match self {
            EffectMode::Static => "static",
            EffectMode::Breathing => "breathing",
            EffectMode::Strobing => "strobing",
            EffectMode::Audio => "audio",
            EffectMode::Battery => "battery",
            EffectMode::Off => "off",
        }
    }

    pub fn from_name(name: &str) -> Option<EffectMode> {
        match name {
            "static" => Some(EffectMode::Static),
            "breathing" => Some(EffectMode::Breathing),
            "strobing" => Some(EffectMode::Strobing),
            "audio" => Some(EffectMode::Audio),
            "battery" => Some(EffectMode::Battery),
            "off" => Some(EffectMode::Off),
            _ => None,
        }
    }
}

/// Live effect tunables. Loops re-read these every iteration, so sliders
/// respond instantly without restarting the effect thread.
#[derive(Debug)]
pub struct EffectParams {
    pub power: AtomicBool,
    pub mode: StdMutex<EffectMode>,
    pub speed: AtomicU8,              // 1..=10
    pub sensitivity: AtomicU32,       // 0..=100
    pub static_level: AtomicU8,       // 0..=2
    pub battery_threshold: AtomicU32, // percent 1..=100
    pub tray_close: AtomicBool,
}

impl Default for EffectParams {
    fn default() -> Self {
        Self {
            power: AtomicBool::new(false),
            mode: StdMutex::new(EffectMode::Breathing),
            speed: AtomicU8::new(4),
            sensitivity: AtomicU32::new(60),
            static_level: AtomicU8::new(BACKLIGHT_HIGH),
            battery_threshold: AtomicU32::new(20),
            tray_close: AtomicBool::new(true),
        }
    }
}

/// Map the 1..=10 UI speed to a per-step interval, hard-floored at 100 ms.
pub fn interval_from_speed(speed: u8) -> u64 {
    let speed = speed.clamp(1, 10) as u64;
    (500 - (speed - 1) * 44).clamp(100, crate::driver::MAX_INTERVAL_MS)
}

/// Sleep no less than 100 ms — the EC write budget is respected everywhere.
pub fn capped_sleep(interval_ms: u64) {
    thread::sleep(std::time::Duration::from_millis(interval_ms.max(100)));
}

/// Everything an effect loop needs; shared across effect threads.
pub struct EffectShared {
    pub driver: DriverMutex,
    pub params: Arc<EffectParams>,
    pub error: StdMutex<Option<String>>,
}

impl EffectShared {
    pub fn report_error(&self, msg: String) {
        eprintln!("[glowfx] {msg}");
        if let Ok(mut e) = self.error.lock() {
            *e = Some(msg);
        }
    }

    /// Quantize then write. Locking is brief (a single sysfs/ACPI write).
    pub fn set_level(&self, level: u8) {
        let level = level.clamp(BACKLIGHT_OFF, BACKLIGHT_HIGH);
        if let Ok(driver) = self.driver.lock() {
            match driver.set_brightness(level) {
                Ok(()) => {}
                Err(e) => self.report_error(e),
            }
        }
    }
}

struct Control {
    _stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

/// Singleton gatekeeper guaranteeing only ONE effect loop / manual command
/// touches the hardware at any given moment.
pub struct EffectManager {
    _driver: DriverMutex,
    params: Arc<EffectParams>,
    shared: Arc<EffectShared>,
    active: StdMutex<Option<Control>>,
}

impl EffectManager {
    pub fn new(driver: Box<dyn BacklightDriver>) -> Arc<Self> {
        let params = Arc::new(EffectParams::default());
        let driver = Arc::new(StdMutex::new(driver));
        let shared = Arc::new(EffectShared {
            driver: Arc::clone(&driver),
            params: Arc::clone(&params),
            error: StdMutex::new(None),
        });
        Arc::new(Self {
            _driver: driver,
            params,
            shared,
            active: StdMutex::new(None),
        })
    }

    pub fn params(&self) -> Arc<EffectParams> {
        Arc::clone(&self.params)
    }

    pub fn driver_info(&self) -> (bool, String) {
        let d = self._driver.lock().unwrap();
        (d.is_supported(), d.describe())
    }

    pub fn last_error(&self) -> Option<String> {
        self.shared.error.lock().unwrap().clone()
    }

    fn stop_current(&self) {
        let ctrl = self.active.lock().unwrap().take();
        if let Some(ctrl) = ctrl {
            ctrl._stop.store(true, Ordering::Relaxed);
            if let Some(h) = ctrl.handle {
                let _ = h.join();
            }
        }
    }

    /// Stop the running effect and start the one implied by params/power.
    pub fn apply(&self) {
        self.stop_current();

        let mode = *self.params.mode.lock().unwrap();
        let power = self.params.power.load(Ordering::Relaxed);
        let mode = if power { mode } else { EffectMode::Off };

        let stop = Arc::new(AtomicBool::new(false));
        let shared = Arc::clone(&self.shared);
        let stop_for_loop = Arc::clone(&stop);
        let handle = thread::spawn(move || run_effect(shared, stop_for_loop, mode));

        *self.active.lock().unwrap() = Some(Control {
            _stop: stop,
            handle: Some(handle),
        });
    }

    /// Hard stop for exit: join the loop so no thread touches the EC afterwards.
    pub fn stop_all(&self) {
        self.stop_current();
    }

    /// Graceful shutdown — stop everything, then restore a sane brightness.
    pub fn shutdown(&self, restore: bool) {
        self.stop_current();
        if restore {
            self.shared.set_level(BACKLIGHT_HIGH);
        }
    }
}

fn run_effect(shared: Arc<EffectShared>, stop: Arc<AtomicBool>, mode: EffectMode) {
    match mode {
        EffectMode::Off => run_off(&shared, &stop),
        EffectMode::Static => run_static(&shared, &stop),
        EffectMode::Breathing => breathing::run(&shared, &stop),
        EffectMode::Strobing => strobing::run(&shared, &stop),
        EffectMode::Audio => audio::run(Arc::clone(&shared), Arc::clone(&stop)),
        EffectMode::Battery => battery::run(Arc::clone(&shared), Arc::clone(&stop)),
    }
}

/// Power off: write 0 once, then idle the loop.
fn run_off(shared: &EffectShared, stop: &AtomicBool) {
    shared.set_level(BACKLIGHT_OFF);
    while !stop.load(Ordering::Relaxed) {
        crate::effects::capped_sleep(250);
    }
}

/// Static: keep the chosen level armed (recovers from manual brightness bumps).
fn run_static(shared: &EffectShared, stop: &AtomicBool) {
    while !stop.load(Ordering::Relaxed) {
        let l = shared
            .params
            .static_level
            .load(Ordering::Relaxed)
            .clamp(BACKLIGHT_OFF, BACKLIGHT_HIGH);
        shared.set_level(l);
        crate::effects::capped_sleep(300);
    }
}

pub fn is_stopped(stop: &AtomicBool) -> bool {
    stop.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_is_hard_capped_at_100ms() {
        assert!(interval_from_speed(10) >= 100);
        assert!(interval_from_speed(1) <= 500);
        for s in 1..=10 {
            assert!(interval_from_speed(s) >= 100, "speed {s} must be ≥100ms");
        }
    }

    #[test]
    fn mode_names_round_trip() {
        for m in [
            EffectMode::Static,
            EffectMode::Breathing,
            EffectMode::Strobing,
            EffectMode::Audio,
            EffectMode::Battery,
            EffectMode::Off,
        ] {
            assert_eq!(EffectMode::from_name(m.name()), Some(m));
        }
        assert_eq!(EffectMode::from_name("nope"), None);
    }
}