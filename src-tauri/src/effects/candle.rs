//! Candle/ember flicker: mostly mid-level with weighted dips to the floor and
//! occasional brief flares up to the ceiling.

use std::sync::atomic::{AtomicBool, Ordering};

use super::{capped_sleep, interval_from_speed, is_stopped, level_range, EffectShared, Rng};

pub fn run(shared: &EffectShared, stop: &AtomicBool) {
    let mut rng = Rng::new();
    let mut settle = 0u32; // ticks to hold the mid level after a flare

    while !is_stopped(stop) {
        let (min, max) = level_range(&shared.params);
        let mid = min + (max.saturating_sub(min) + 1) / 2;

        let level = if settle > 0 {
            settle -= 1;
            mid
        } else {
            match rng.below(100) {
                0..=5 => {
                    settle = rng.below(3) as u32 + 1;
                    min
                }
                6..=14 => max, // brief flare
                _ => mid,
            }
        };
        shared.emit(level);

        let interval = interval_from_speed(shared.params.speed.load(Ordering::Relaxed));
        capped_sleep(interval);
    }
}