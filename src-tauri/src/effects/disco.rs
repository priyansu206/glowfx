//! Disco/random bursts: weighted random jumps across the reachable levels.

use std::sync::atomic::{AtomicBool, Ordering};

use super::{capped_sleep, interval_from_speed, is_stopped, level_range, EffectShared, Rng};

pub fn run(shared: &EffectShared, stop: &AtomicBool) {
    let mut rng = Rng::new();

    while !is_stopped(stop) {
        let (min, max) = level_range(&shared.params);
        let mid = min + (max.saturating_sub(min) + 1) / 2;

        // Heavy bias toward the extremes for a "party" feel.
        let r = rng.below(100);
        let level = match r {
            0..=45 => max,
            46..=70 => min,
            _ => mid,
        };
        shared.emit(level);

        let interval = interval_from_speed(shared.params.speed.load(Ordering::Relaxed));
        capped_sleep(interval);
    }
}