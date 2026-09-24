//! Strobing effect: high/low flash toggle bounded by the 10 Hz cap.
//! Waveform 0 = square (min/max toggle); waveform 1 = decay triangle ramp.

use std::sync::atomic::{AtomicBool, Ordering};

use super::{capped_sleep, interval_from_speed, is_stopped, level_range, EffectShared};

pub fn run(shared: &EffectShared, stop: &AtomicBool) {
    let mut on = true;
    let mut phase = 0u32;

    while !is_stopped(stop) {
        let (min, max) = level_range(&shared.params);
        let speed = shared.params.speed.load(Ordering::Relaxed);
        let interval = interval_from_speed(speed);
        let waveform = shared.params.waveform.load(Ordering::Relaxed);

        let level = if waveform == 1 {
            let width = (u32::from(max) - u32::from(min) + 1) as u32; // distinct levels
            let period = 2 * width - 1;
            let n = ((phase + width - 1) % period) as i64; // 0..period-1, offset to peak first
            let dist = (n - (width as i64 - 1)).abs(); // 0 .. width-1
            phase += 1;
            (max as i64 - dist).clamp(i64::from(min), i64::from(max)) as u8
        } else {
            let l = if on { max } else { min };
            on = !on;
            l
        };

        shared.emit(level);
        capped_sleep(interval);
    }
}