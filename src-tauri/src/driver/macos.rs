//! macOS: no supported raw backlight interface for Lenovo keyboard zones.
//! Safe fallback — the trait degrades gracefully and reports `not supported`
//! instead of attempting to poke IOKit / SMC registers it does not own.

use super::{BacklightDriver, NullDriver};

pub fn detect() -> Box<dyn BacklightDriver> {
    Box::new(NullDriver::new(
        "macOS is not supported by GlowFX (no public Lenovo backlight ACPI interface)".to_string(),
    ))
}