//! Blink effect: rhythmic square toggle between the floor and the ceiling.
//! Waveform 0 = square on/off; waveform 1 = decaying staircase (max..min).

use std::sync::atomic::{AtomicBool, Ordering};

use super::{capped_sleep, interval_from_speed, is_stopped, level_range, EffectShared};

pub fn run(shared: &EffectShared, stop: &AtomicBool) {
    let mut on = true;
    let mut decay = 0u8;

    while !is_stopped(stop) {
        let (min, max) = level_range(&shared.params);
        let waveform = shared.params.waveform.load(Ordering::Relaxed);

        let level = if waveform == 1 {
            // decay: step from max down to min, shortest ramp possible on {0,1,2}
            decay = if on { max.min(2) } else { decay.saturating_sub(1) };
            if on && decay <= min {
                on = false;
            }
            if !on && decay <= min {
                on = true;
            }
            decay
        } else if on {
            max
        } else {
            min
        };
        shared.emit(level);

        if waveform != 1 {
            on = !on;
        }
        let interval = interval_from_speed(shared.params.speed.load(Ordering::Relaxed));
        capped_sleep(interval);
    }
}