//! Idle effect: fades the backlight to its floor when the session has been
//! idle for `idle_grace_s` and wakes it back to the ceiling on activity.
//! Idle time comes from org.freedesktop.ScreenSaver via DBus; if the session
//! bus is unavailable the effect safely no-ops (stays at the ceiling).

use std::sync::atomic::{AtomicBool, Ordering};

use super::{capped_sleep, is_stopped, level_range, EffectShared};

const POLL_MS: u64 = 1_000;

pub fn run(shared: &EffectShared, stop: &AtomicBool) {
    let mut reported = false;

    while !is_stopped(stop) {
        match session_idle_seconds() {
            Some(idle) => {
                reported = false;
                let grace = shared.params.idle_grace_s.load(Ordering::Relaxed).max(15) as u64;
                let (min, max) = level_range(&shared.params);
                shared.emit(if idle >= grace { min } else { max });
            }
            None => {
                if !reported {
                    reported = true;
                    shared.report_error(
                        "idle detection unavailable (no session DBus); staying on".into(),
                    );
                }
            }
        }
        capped_sleep(POLL_MS);
    }
}

/// Session idle seconds via org.freedesktop.ScreenSaver.GetSessionIdleTime.
fn session_idle_seconds() -> Option<u64> {
    if std::env::var("DBUS_SESSION_BUS_ADDRESS").is_err() {
        return None;
    }
    let conn = zbus::blocking::Connection::session().ok()?;
    let msg = conn
        .call_method(
            Some("org.freedesktop.ScreenSaver"),
            "/org/freedesktop/ScreenSaver",
            Some("org.freedesktop.ScreenSaver"),
            "GetSessionIdleTime",
            &(),
        )
        .ok()?;
    let idle: u32 = msg.body().deserialize().ok()?;
    Some(idle as u64)
}