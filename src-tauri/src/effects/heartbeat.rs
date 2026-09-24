//! Heartbeat effect: a "lub-dub" double thump over a fixed pattern frame.

use std::sync::atomic::{AtomicBool, Ordering};

use super::{capped_sleep, interval_from_speed, is_stopped, EffectShared};

/// Per-step level indices over one full frame (10 steps, 0 = rest/mid).
/// Order: [rest, rest, rest, THUMP, echo, THUMP, echo, rest, rest, rest]
const PATTERN: [u8; 10] = [0, 0, 0, 3, 2, 3, 2, 0, 0, 0];

pub fn run(shared: &EffectShared, stop: &AtomicBool) {
    let mut step = 0u32;

    while !is_stopped(stop) {
        let (min, max) = {
            let mi = shared.params.min_level.load(Ordering::Relaxed).clamp(0, 2);
            let ma = shared.params.max_level.load(Ordering::Relaxed).clamp(0, 2);
            (mi.min(ma), ma.max(mi))
        };
        let mid = min + (max.saturating_sub(min) + 1) / 2;

        let level = match PATTERN[(step % 10) as usize] {
            0 => min,
            1 => max,
            _ => mid,
        };
        shared.emit(level);

        step += 1;
        let interval = interval_from_speed(shared.params.speed.load(Ordering::Relaxed));
        capped_sleep(interval);
    }
}