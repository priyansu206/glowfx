//! Schedule effect: brightness follows the time of day. The "day" window runs
//! from `day_start` to `night_start` around the 24 h clock (wrapping at midnight);
//! the two boundary hours render at the mid level, the night floor otherwise.

use std::sync::atomic::{AtomicBool, Ordering};

use chrono::Timelike;

use super::{capped_sleep, is_stopped, level_range, EffectShared};

const CHECK_MS: u64 = 15_000;

pub fn run(shared: &EffectShared, stop: &AtomicBool) {
    let mut last = None;

    while !is_stopped(stop) {
        let hour = chrono::Local::now().hour() as u8;
        if last != Some(hour) {
            last = Some(hour);
            let (min, max) = level_range(&shared.params);
            let mid = min + (max.saturating_sub(min) + 1) / 2;

            let day = shared.params.day_start_hour.load(Ordering::Relaxed) % 24;
            let night = shared.params.night_start_hour.load(Ordering::Relaxed) % 24;

            let bright = hour == day || (day == night) || if day < night {
                hour >= day && hour < night
            } else {
                hour >= day || hour < night
            };
            let level = if bright {
                if hour == day || hour == night { mid } else { max }
            } else {
                min
            };
            shared.emit(level);
        }

        capped_sleep(CHECK_MS);
    }
}