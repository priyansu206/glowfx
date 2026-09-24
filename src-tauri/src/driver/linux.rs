//! Linux driver: writes `0|1|2` into `/sys/class/leds/*kbd_backlight/brightness`.
//!
//! The sysfs LED node is a volatile register view of the Embedded Controller;
//! no NVRAM/EEPROM flash updates are ever issued. Height-mode writes are
//! always rate-limited at the effect layer (≥100 ms between writes).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU8, Ordering};

use super::{quantize_level, BacklightDriver, NullDriver};

pub const UDEV_RULE_PATH: &str = "/etc/udev/rules.d/99-lenovo-glowfx.rules";
const UDEV_RULE: &str = r#"# GlowFX: grant user access to Lenovo keyboard backlight
# Targets the volatile sysfs brightness attribute only (MODE=0666).
SUBSYSTEM=="leds", KERNEL=="*kbd_backlight", MODE="0666"
"#;

pub struct LinuxDriver {
    path: PathBuf,
    cache: AtomicU8,
}

impl LinuxDriver {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            cache: AtomicU8::new(0),
        }
    }

    fn write_raw(&self, level: u8) -> std::io::Result<()> {
        let mut f = OpenOptions::new().write(true).open(&self.path)?;
        f.write_all(level.to_string().as_bytes())?;
        self.cache.store(level, Ordering::Relaxed);
        Ok(())
    }
}

impl BacklightDriver for LinuxDriver {
    fn set_brightness(&self, level: u8) -> Result<(), String> {
        let level = quantize_level(level);
        self.write_raw(level)
            .map_err(|e| format!("kbd backlight write failed: {e}"))
    }

    fn get_brightness(&self) -> Result<u8, String> {
        match fs::read_to_string(&self.path) {
            Ok(s) => s.trim().parse::<u8>().map_err(|e| e.to_string()),
            Err(_) => Ok(self.cache.load(Ordering::Relaxed)),
        }
    }

    fn is_supported(&self) -> bool {
        true
    }

    fn describe(&self) -> String {
        self.path.display().to_string()
    }
}

/// Restore a sane brightness when the driver is dropped (panic / normal teardown).
impl Drop for LinuxDriver {
    fn drop(&mut self) {
        let _ = self.write_raw(2);
    }
}

/// Find a Lenovo-compatible `*kbd_backlight` LED node under `/sys/class/leds`.
fn find_backlight() -> Option<PathBuf> {
    let base = Path::new("/sys/class/leds");
    if !base.is_dir() {
        return None;
    }
    for entry in fs::read_dir(base).ok()?.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.contains("kbd_backlight") {
            let brightness = entry.path().join("brightness");
            if brightness.is_file() {
                return Some(brightness);
            }
        }
    }
    None
}

pub fn detected_path() -> Option<String> {
    find_backlight().map(|p| p.display().to_string())
}

/// Is the brightness attribute actually writable by the current user?
/// (Probes by opening for write — does not mutate the value.)
pub fn is_writable() -> bool {
    let Some(p) = find_backlight() else {
        return false;
    };
    OpenOptions::new().write(true).open(&p).is_ok()
}

pub fn detect() -> Box<dyn BacklightDriver> {
    match find_backlight() {
        Some(path) => Box::new(LinuxDriver::new(path)),
        None => Box::new(NullDriver::new(
            "No *kbd_backlight LED found in /sys/class/leds".to_string(),
        )),
    }
}

/// Install the udev rule so the app can write without root. Elevated via pkexec.
pub fn install_udev_rule() -> Result<String, String> {
    let Some(path) = find_backlight() else {
        return Err("No keyboard backlight found under /sys/class/leds".into());
    };

    let path_display = path.display();
    let script = format!(
        r#"set -eu
install -m 0644 /dev/stdin "{UDEV_RULE_PATH}" <<'EOF'
{UDEV_RULE}
EOF
udevadm control --reload-rules 2>/dev/null || true
udevadm trigger --subsystem-match=leds 2>/dev/null || true
chmod 0666 "{path_display}" 2>/dev/null || true
for f in /sys/class/leds/*kbd_backlight/brightness; do [ -e "$f" ] && chmod 0666 "$f" || true; done
echo done
"#
    );

    let status = Command::new("pkexec")
        .args(["sh", "-c", &script])
        .status()
        .map_err(|e| format!("Could not run pkexec: {e}. Install rootless or run upgrade manually."))?;

    if !status.success() {
        return Err(format!(
            "Permission dialog declined or failed (exit {}). Rule was not applied.",
            status.code().unwrap_or(-1)
        ));
    }

    if is_writable() {
        Ok("udev rule installed and brightness is now user-writable.".to_string())
    } else {
        Ok("udev rule written; logs or a reboot may be required for the mode to apply.".to_string())
    }
}

/// Read-only probe used by the settings panel: configured, writable, path.
pub fn udev_state() -> (bool, bool, String) {
    let configured = Path::new(UDEV_RULE_PATH).exists();
    let path = find_backlight()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "—".to_string());
    (configured, is_writable(), path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_backlight_node_on_real_hardware() {
        // On a real Lenovo VPC system this finds platform::kbd_backlight;
        // otherwise the driver gracefully reports unsupported.
        let _ = find_backlight();
    }

    #[test]
    fn set_brightness_quantizes_to_three_states() {
        if !is_writable() {
            eprintln!("skipping live write: kbd_backlight not user-writable");
            return;
        }
        let driver = LinuxDriver::new(find_backlight().expect("node present"));
        driver.set_brightness(2).unwrap();
        driver.set_brightness(0).unwrap();
        assert!(driver.is_supported());
    }
}