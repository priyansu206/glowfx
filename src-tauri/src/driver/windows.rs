//! Windows driver: `root\WMI` -> `Lenovo_SetBacklight.SetBacklight(Level)`.
//!
//! Talks natively to WMI through the `wmi` crate — no PowerShell/cmd is ever
//! spawned, including inside effect loops. A WMI session is kept alive
//! per-thread (persistent COMLibrary lease), so the 5–10 Hz effect loops reuse
//! one connection instead of building a fresh one for every write.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};

use super::{quantize_level, BacklightDriver, NullDriver};

const WMI_NAMESPACE: &str = "ROOT\\WMI";

thread_local! {
    /// One persistent connection per thread (WMIConnection is !Send / !Sync
    /// because it wraps COM, so it must live thread-locally).
    static WMI_CONN: RefCell<Option<Result<wmi::WMIConnection, String>>> = const { RefCell::new(None) };
}

/// Lazily obtain (and then reuse) the thread's WMI session.
fn conn_clone() -> Result<wmi::WMIConnection, String> {
    WMI_CONN.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = Some(
                wmi::WMIConnection::with_namespace_path(WMI_NAMESPACE)
                    .map_err(|e| format!("WMI connect failed: {e:?}")),
            );
        }
        slot.as_ref().unwrap().clone()
    })
}

#[derive(Debug, Deserialize)]
struct Lenovo_SetBacklight {
    #[serde(rename = "__Path")]
    path: String,
}

#[derive(Debug, Serialize)]
struct SetBacklightArgs {
    Level: u8,
}

/// The Lenovo WMI backlight endpoint is exposed as a singleton instance of
/// `Lenovo_SetBacklight` in `root\WMI`.
fn exec_set(level: u8) -> Result<(), String> {
    let c = conn_clone()?;
    let instances: Vec<Lenovo_SetBacklight> = c
        .raw_query("SELECT __Path FROM Lenovo_SetBacklight")
        .map_err(|e| format!("query Lenovo_SetBacklight failed: {e:?}"))?;

    let Some(inst) = instances.first() else {
        return Err(
            "Lenovo backlight WMI endpoint not present (no Lenovo_SetBacklight instance)".into(),
        );
    };

    // If the method returns void and has no out params, Out must be `()`.
    c.exec_instance_method::<Lenovo_SetBacklight, ()>(
        &inst.path,
        "SetBacklight",
        SetBacklightArgs { Level: level },
    )
    .map_err(|e| format!("SetBacklight({level}) failed: {e:?}"))?;
    Ok(())
}

pub struct WindowsDriver {
    cache: AtomicU8,
}

impl WindowsDriver {
    pub fn new() -> Self {
        Self {
            cache: AtomicU8::new(2),
        }
    }
}

impl BacklightDriver for WindowsDriver {
    fn set_brightness(&self, level: u8) -> Result<(), String> {
        let level = quantize_level(level);
        exec_set(level)?;
        self.cache.store(level, Ordering::Relaxed);
        Ok(())
    }

    fn get_brightness(&self) -> Result<u8, String> {
        Ok(self.cache.load(Ordering::Relaxed))
    }

    fn is_supported(&self) -> bool {
        true
    }

    fn describe(&self) -> String {
        "root\\WMI:Lenovo_SetBacklight".to_string()
    }
}

pub fn detect() -> Box<dyn BacklightDriver> {
    // Probe once up front so unsupported machines get a graceful status.
    match conn_clone().and_then(validate_endpoint) {
        Ok(()) => Box::new(WindowsDriver::new()),
        Err(e) => Box::new(NullDriver::new(e)),
    }
}

fn validate_endpoint(c: wmi::WMIConnection) -> Result<(), String> {
    let instances: Vec<Lenovo_SetBacklight> = c
        .raw_query("SELECT __Path FROM Lenovo_SetBacklight")
        .map_err(|e| format!("Lenovo backlight WMI class unavailable: {e:?}"))?;
    if instances.is_empty() {
        Err("No Lenovo_SetBacklight instance found; unsupported or driver absent.".into())
    } else {
        Ok(())
    }
}