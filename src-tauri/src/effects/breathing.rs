//! Breathing effect: smooth quantized sine wave on the discrete {0,1,2} scale.

use std::sync::atomic::{AtomicBool, Ordering};

use super::{capped_sleep, interval_from_speed, is_stopped, EffectShared};

/// Fixed phase advance per step (degrees). A full sine cycle is 360°, so
/// slower intervals produce longer, smoother fade cycles while staying within
/// the discrete {0,1,2} scale and the 10 Hz write cap.
const STEP_DEG: f32 = 24.0;

pub fn run(shared: &EffectShared, stop: &AtomicBool) {
    let mut phase: f32 = 90.0; // start bright (sin(90°) = 1.0)

    while !is_stopped(stop) {
        // Normalize sine to [0,1], scale to [0,2], round to the nearest state.
        let f = ((phase.to_radians().sin() + 1.0) / 2.0).clamp(0.0, 1.0);
        let level = ((f * 2.0).round() as u8).clamp(0, 2);
        shared.set_level(level);

        phase += STEP_DEG;
        if phase >= 360.0 {
            phase %= 360.0;
        }

        let interval = interval_from_speed(shared.params.speed.load(Ordering::Relaxed));
        capped_sleep(interval);
    }
}