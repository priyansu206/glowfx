//! Storm effect: long quiet stretches broken by sudden, clustered lightning
//! bursts — a couple of rapid max-level hops with occasional pre-strike
//! flickers. Distinct from candle flicker (no mid-level settle bias).

use std::sync::atomic::{AtomicBool, Ordering};

use super::{capped_sleep, interval_from_speed, is_stopped, level_range, EffectShared, Rng};

pub fn run(shared: &EffectShared, stop: &AtomicBool) {
    let mut rng = Rng::new();
    let mut quiet_left = 0u32;
    let mut flashes_left = 0u32;

    while !is_stopped(stop) {
        let (min, max) = level_range(&shared.params);

        let level = if flashes_left > 0 {
            flashes_left -= 1;
            max
        } else if quiet_left > 0 {
            quiet_left -= 1;
            if rng.below(100) < 6 {
                max // pre-strike flicker hint
            } else {
                min
            }
        } else {
            // A strike arrives: brief rapid flash cluster, then a long lull.
            flashes_left = rng.below(3) as u32 + 1;
            quiet_left = rng.below(45) as u32 + 5;
            max
        };
        shared.emit(level);

        let interval = interval_from_speed(shared.params.speed.load(Ordering::Relaxed));
        capped_sleep(interval);
    }
}