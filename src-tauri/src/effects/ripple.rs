//! Ripple effect: a sonar/radar-style "ping" — sharp flash up to the ceiling,
//! stepped decay down to the floor, then a long quiet dwell before the next
//! ping. A temporal stand-in for spatial ripples on single-zone hardware.

use std::sync::atomic::{AtomicBool, Ordering};

use super::{capped_sleep, interval_from_speed, is_stopped, level_range, EffectShared};

/// Ticks the flash holds its peak level.
const FLASH_T: i64 = 2;
/// Ticks per decay step, then a long quiet dwell at the floor.
const STEP_T: i64 = 2;
const DWELL_T: i64 = 10;

pub fn run(shared: &EffectShared, stop: &AtomicBool) {
    let mut tick = 0i64;

    while !is_stopped(stop) {
        let (min, max) = level_range(&shared.params);
        let span = i64::from(max - min);
        let period = FLASH_T + span * STEP_T + DWELL_T;
        let t = tick % period;

        let level = if span == 0 {
            max
        } else if t < FLASH_T {
            max
        } else if t < FLASH_T + span * STEP_T {
            let down = (t - FLASH_T) / STEP_T;
            (i64::from(max) - (down + 1)).max(i64::from(min)) as u8
        } else {
            min
        };
        shared.emit(level);

        tick += 1;
        let interval = interval_from_speed(shared.params.speed.load(Ordering::Relaxed));
        capped_sleep(interval);
    }
}