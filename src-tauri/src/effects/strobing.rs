//! Strobing effect: high/low flash toggle, still bounded by the 10 Hz cap.

use std::sync::atomic::{AtomicBool, Ordering};

use super::{capped_sleep, interval_from_speed, is_stopped, EffectShared};

pub fn run(shared: &EffectShared, stop: &AtomicBool) {
    let mut on = true;

    while !is_stopped(stop) {
        let speed = shared.params.speed.load(Ordering::Relaxed);
        let interval = interval_from_speed(speed);

        shared.set_level(if on { 2 } else { 0 });
        on = !on;

        capped_sleep(interval);
    }
}