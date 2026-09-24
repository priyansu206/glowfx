//! Hardware Abstraction Layer (HAL).

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(not(target_os = "linux"))]
pub mod macos;

use std::sync::Mutex;

/// Maximum write frequency to the EC / sysfs backlight controller.
/// Lenovo single-zone controllers only accept 3 discrete states and the
/// LPC/ACPI bus must not be flooded — hard-capped at 10 Hz (≥100 ms).
pub const MIN_INTERVAL_MS: u64 = 100;
/// Soft upper bound for effect interval (slow effects still sleep at least 100 ms).
pub const MAX_INTERVAL_MS: u64 = 2000;

/// Clamp a raw level into the only states the controller understands: 0/1/2.
pub fn quantize_level(level: u8) -> u8 {
    level.clamp(0, 2)
}

/// Unified, cross-platform backlight driver.
pub trait BacklightDriver: Send + Sync {
    /// Write one of {0, 1, 2} into the volatile brightness register.
    fn set_brightness(&self, level: u8) -> Result<(), String>;
    fn get_brightness(&self) -> Result<u8, String>;
    fn is_supported(&self) -> bool;
    fn describe(&self) -> String;
}

/// Returned when no usable backlight interface could be found.
#[derive(Default)]
pub struct NullDriver {
    pub reason: Mutex<Option<String>>,
}

impl NullDriver {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: Mutex::new(Some(reason.into())),
        }
    }
}

impl BacklightDriver for NullDriver {
    fn set_brightness(&self, _level: u8) -> Result<(), String> {
        let r = self.reason.lock().unwrap();
        Err(r.clone().unwrap_or_else(|| "Driver unavailable".to_string()))
    }
    fn get_brightness(&self) -> Result<u8, String> {
        Err("Driver unavailable".to_string())
    }
    fn is_supported(&self) -> bool {
        false
    }
    fn describe(&self) -> String {
        "none".to_string()
    }
}

/// Probe the platform and return a usable driver (or a graceful `NullDriver`).
pub fn detect_driver() -> Box<dyn BacklightDriver> {
    #[cfg(target_os = "linux")]
    {
        linux::detect()
    }
    #[cfg(target_os = "windows")]
    {
        windows::detect()
    }
    #[cfg(not(target_os = "linux"))]
    {
        macos::detect()
    }
}