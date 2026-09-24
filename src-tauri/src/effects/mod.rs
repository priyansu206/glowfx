//! Effect engine. Runs exactly one loop at a time; every driver write is
//! quantized to {0,1,2} and rate-limited to ≥100 ms (≤10 Hz) to protect the
//! LPC/ACPI bus from flooding (input lag / fan stutter protection).

pub mod audio;
pub mod battery;
pub mod breathing;
pub mod disco;
pub mod idle;
pub mod morse;
pub mod radar;
pub mod ripple;
pub mod schedule;
pub mod storm;
pub mod strobing;

use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicU8, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

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
    Storm,
    Ripple,
    Radar,
    Morse,
    Disco,
    Schedule,
    Idle,
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
            EffectMode::Storm => "storm",
            EffectMode::Ripple => "ripple",
            EffectMode::Radar => "radar",
            EffectMode::Morse => "morse",
            EffectMode::Disco => "disco",
            EffectMode::Schedule => "schedule",
            EffectMode::Idle => "idle",
            EffectMode::Off => "off",
        }
    }

    pub fn from_name(name: &str) -> Option<EffectMode> {
        Some(match name {
            "static" => EffectMode::Static,
            "breathing" => EffectMode::Breathing,
            "strobing" => EffectMode::Strobing,
            "audio" => EffectMode::Audio,
            "battery" => EffectMode::Battery,
            "storm" => EffectMode::Storm,
            "ripple" => EffectMode::Ripple,
            "radar" => EffectMode::Radar,
            "morse" => EffectMode::Morse,
            "disco" => EffectMode::Disco,
            "schedule" => EffectMode::Schedule,
            "idle" => EffectMode::Idle,
            "off" => EffectMode::Off,
            _ => return None,
        })
    }
}

/// Live effect tunables. Loops re-read these every iteration, so sliders
/// respond instantly without restarting the effect thread.
#[derive(Debug)]
pub struct EffectParams {
    pub power: AtomicBool,
    pub mode: StdMutex<EffectMode>,
    pub speed: AtomicU8,               // 1..=10
    pub sensitivity: AtomicU32,        // 0..=100
    pub static_level: AtomicU8,        // 0..=2
    pub battery_threshold: AtomicU32,  // percent 1..=100 (guard dim)
    pub critical_threshold: AtomicU8,  // percent 1..=50 (SOS blink)
    pub min_level: AtomicU8,           // per-zone floor for fade modes
    pub max_level: AtomicU8,           // per-zone ceiling for flash modes
    pub waveform: AtomicU8,            // 0 = square, 1 = decay
    pub audio_beat: AtomicBool,        // beat-sync instead of level thresholds
    pub auto_dim_minutes: AtomicU16,   // 0 = disabled; dim to off after N min
    pub day_start_hour: AtomicU8,      // schedule: bright window start (0..=23)
    pub night_start_hour: AtomicU8,    // schedule: dim floor start (0..=23)
    pub idle_grace_s: AtomicU16,       // idle effect: seconds before fade (>=15)
    pub tray_close: AtomicBool,
    pub morse_text: StdMutex<String>,  // morse effect: message to transmit
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
            critical_threshold: AtomicU8::new(10),
            min_level: AtomicU8::new(BACKLIGHT_OFF),
            max_level: AtomicU8::new(BACKLIGHT_HIGH),
            waveform: AtomicU8::new(0),
            audio_beat: AtomicBool::new(false),
            auto_dim_minutes: AtomicU16::new(0),
            day_start_hour: AtomicU8::new(7),
            night_start_hour: AtomicU8::new(22),
            idle_grace_s: AtomicU16::new(60),
            tray_close: AtomicBool::new(true),
            morse_text: StdMutex::new("HI".into()),
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
    thread::sleep(Duration::from_millis(interval_ms.max(100)));
}

/// Per-loop floor/ceiling, clamped and ordered so min <= max regardless of
/// what the user throws at the UI.
pub fn level_range(params: &EffectParams) -> (u8, u8) {
    let min = params.min_level.load(Ordering::Relaxed).clamp(BACKLIGHT_OFF, BACKLIGHT_HIGH);
    let max = params.max_level.load(Ordering::Relaxed).clamp(BACKLIGHT_OFF, BACKLIGHT_HIGH);
    (min.min(max), max.max(min))
}

/// Tiny deterministic PRNG for flicker/burst modes (no external dep, no IO).
pub struct Rng(u64);

impl Rng {
    pub fn new() -> Self {
        let seed = Instant::now()
            .elapsed()
            .as_nanos()
            .to_le_bytes()
            .iter()
            .fold(0x9e37_79b9_7f4a_7c15u64, |acc, &b| acc.wrapping_mul(1099511628211).wrapping_add(b as u64));
        Rng(seed | 1)
    }

    /// Uniform in [0, max).
    pub fn below(&mut self, max: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0.wrapping_rem(max.max(1))
    }
}

/// Everything an effect loop needs; shared across effect threads.
pub struct EffectShared {
    pub driver: DriverMutex,
    pub params: Arc<EffectParams>,
    pub error: StdMutex<Option<String>>,
    started: StdMutex<Instant>,
}

impl EffectShared {
    fn new(driver: DriverMutex, params: Arc<EffectParams>) -> Arc<Self> {
        Arc::new(Self {
            driver,
            params,
            error: StdMutex::new(None),
            started: StdMutex::new(Instant::now()),
        })
    }

    /// Reset the auto-dim clock (called each time a mode is (re)started).
    pub fn mark_start(&self) {
        if let Ok(mut s) = self.started.lock() {
            *s = Instant::now();
        }
    }

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

    /// Effect-level write with the optional auto-dim override applied:
    /// once the mode has run longer than auto_dim_minutes, force level 0.
    pub fn emit(&self, level: u8) {
        let p = &self.params;
        let dim_min = p.auto_dim_minutes.load(Ordering::Relaxed) as u64;
        if dim_min > 0 {
            let elapsed = self.started.lock().map(|s| s.elapsed().as_secs()).unwrap_or(0);
            if elapsed > dim_min * 60 {
                self.set_level(BACKLIGHT_OFF);
                return;
            }
        }
        self.set_level(level);
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
        let shared = EffectShared::new(Arc::clone(&driver), Arc::clone(&params));
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

        let shared = Arc::clone(&self.shared);
        shared.mark_start();

        let stop = Arc::new(AtomicBool::new(false));
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
        EffectMode::Storm => storm::run(&shared, &stop),
        EffectMode::Ripple => ripple::run(&shared, &stop),
        EffectMode::Radar => radar::run(&shared, &stop),
        EffectMode::Morse => morse::run(&shared, &stop),
        EffectMode::Disco => disco::run(&shared, &stop),
        EffectMode::Schedule => schedule::run(&shared, &stop),
        EffectMode::Idle => idle::run(&shared, &stop),
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
        shared.emit(l);
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
            EffectMode::Storm,
            EffectMode::Ripple,
            EffectMode::Radar,
            EffectMode::Morse,
            EffectMode::Disco,
            EffectMode::Schedule,
            EffectMode::Idle,
            EffectMode::Off,
        ] {
            assert_eq!(EffectMode::from_name(m.name()), Some(m));
        }
        assert_eq!(EffectMode::from_name("nope"), None);
    }

    #[test]
    fn level_range_orders_min_max() {
        let p = EffectParams::default();
        p.max_level.store(0, Ordering::Relaxed);
        p.min_level.store(2, Ordering::Relaxed);
        assert_eq!(level_range(&p), (0, 2));
        p.min_level.store(7, Ordering::Relaxed);
        assert_eq!(level_range(&p), (0, 2));
    }

    /// Drive a handful of the new effect loops against the real backlight and
    /// assert they run bounded within {0,1,2} (skipped when no node exists).
    #[test]
    fn new_effect_loops_stay_bounded() {
        use std::time::Duration;

        let driver = crate::driver::detect_driver();
        if !driver.is_supported() {
            eprintln!("skipping live effect loop test (no backlight node)");
            return;
        }
        let params = Arc::new(EffectParams::default());
        let shared = EffectShared::new(Arc::new(StdMutex::new(driver)), params);

        let runs: [(&str, fn(&EffectShared, &AtomicBool)); 5] = [
            ("storm", storm::run),
            ("ripple", ripple::run),
            ("radar", radar::run),
            ("morse", morse::run),
            ("disco", disco::run),
        ];
        for (name, run) in runs {
            let s = Arc::clone(&shared);
            let stop = Arc::new(AtomicBool::new(false));
            let st = Arc::clone(&stop);
            let h = thread::spawn(move || run(&s, &st));
            thread::sleep(Duration::from_millis(700));
            stop.store(true, Ordering::Relaxed);
            let _ = h.join();
            if let Ok(raw) =
                std::fs::read_to_string("/sys/class/leds/platform::kbd_backlight/brightness")
            {
                let v: u8 = raw.trim().parse().unwrap_or(9);
                assert!(v <= BACKLIGHT_HIGH, "{name} wrote out-of-range {v}");
            }
        }
        shared.set_level(BACKLIGHT_HIGH);
    }
}