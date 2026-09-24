//! Radar effect: simulate a rotating radar sweep on a single-zone backlight.
//! A bright leading edge sweeps up, the level trails off as the arm rotates,
//! and a random mid-sweep "blip" marks a detected target. Continuous rotations
//! with no long dwell — distinct from Ripple's single ping + silence.

use std::sync::atomic::{AtomicBool, Ordering};

use super::{capped_sleep, interval_from_speed, is_stopped, level_range, EffectShared, Rng};

/// Ticks the bright leading edge holds.
const RISE_T: u32 = 2;
/// Ticks the trailing decay spans (slow fade of the sweep arm).
const FALL_T: u32 = 6;
/// Ticks the arm sits at the floor between rotations.
const GAP_T: u32 = 4;

pub fn run(shared: &EffectShared, stop: &AtomicBool) {
    let mut rng = Rng::new();
    let mut tick = 0u64;
    let mut blip_at = 0u32;

    while !is_stopped(stop) {
        let (min, max) = level_range(&shared.params);
        let span = i64::from(max - min);
        let period = RISE_T + FALL_T + GAP_T;
        let t = (tick % period as u64) as u32;

        // At the start of each rotation, decide where the next blip lands.
        if t == 0 {
            blip_at = rng.below(FALL_T as u64) as u32;
        }

        let level = if span == 0 {
            max
        } else if t < RISE_T {
            max // leading edge of the sweep
        } else if t < RISE_T + FALL_T {
            let down = (t - RISE_T) as i64;
            let trail = (i64::from(max) - down / 2).max(i64::from(min)) as u8;
            if (t - RISE_T) == blip_at {
                max // target blip flash on the trailing arm
            } else {
                trail
            }
        } else {
            min // quiet gap before the next rotation
        };
        shared.emit(level);

        tick += 1;
        let interval = interval_from_speed(shared.params.speed.load(Ordering::Relaxed));
        capped_sleep(interval);
    }
}