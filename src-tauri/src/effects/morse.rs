//! Morse Code effect: translate a user-supplied message into on/off pulses.
//! Dot = 1 time unit on, dash = 3 units on, intra-char gap = 1, inter-char
//! gap = 3, inter-word gap = 7. The time unit scales with the speed slider.

use std::sync::atomic::{AtomicBool, Ordering};

use super::{capped_sleep, interval_from_speed, is_stopped, level_range, EffectShared};

const MORSE: [(&str, &str); 36] = [
    ("A", ".-"), ("B", "-..."), ("C", "-.-."), ("D", "-.."), ("E", "."),
    ("F", "..-."), ("G", "--."), ("H", "...."), ("I", ".."), ("J", ".---"),
    ("K", "-.-"), ("L", ".-.."), ("M", "--"), ("N", "-."), ("O", "---"),
    ("P", ".--."), ("Q", "--.-"), ("R", ".-."), ("S", "..."), ("T", "-"),
    ("U", "..-"), ("V", "...-"), ("W", ".--"), ("X", "-..-"), ("Y", "-.--"),
    ("Z", "--.."), ("0", "-----"), ("1", ".----"), ("2", "..---"), ("3", "...--"),
    ("4", "....-"), ("5", "....."), ("6", "-...."), ("7", "--..."), ("8", "---.."),
    ("9", "----."),
];

fn code_for(c: char) -> Option<&'static str> {
    MORSE.iter().find(|(ch, _)| ch.chars().next() == Some(c)).map(|(_, code)| *code)
}

/// Render the message as a run-length-encoded pattern of (on?, unit duration).
fn build_pattern(text: &str) -> Vec<(bool, u32)> {
    let mut units: Vec<bool> = Vec::new();
    let words: Vec<String> = text
        .trim()
        .to_uppercase()
        .split_whitespace()
        .map(|w| w.to_string())
        .collect();

    for (wi, word) in words.iter().enumerate() {
        let chars: Vec<char> = word.chars().collect();
        for (ci, c) in chars.iter().enumerate() {
            let Some(code) = code_for(*c) else { continue };
            if ci > 0 {
                units.extend([false, false, false]); // inter-char gap
            }
            let signs: Vec<char> = code.chars().collect();
            for (si, s) in signs.iter().enumerate() {
                match s {
                    '.' => units.push(true),
                    '-' => units.extend([true, true, true]),
                    _ => {}
                }
                if si + 1 < signs.len() {
                    units.push(false); // intra-char gap
                }
            }
        }
        if wi + 1 < words.len() {
            units.extend([false; 7]); // inter-word gap
        }
    }

    if units.is_empty() {
        return vec![(true, 1), (false, 3)];
    }

    let mut out = Vec::new();
    let mut cur = units[0];
    let mut n = 1u32;
    for &u in &units[1..] {
        if u == cur {
            n += 1;
        } else {
            out.push((cur, n));
            cur = u;
            n = 1;
        }
    }
    out.push((cur, n));
    out
}

pub fn run(shared: &EffectShared, stop: &AtomicBool) {
    let pattern = build_pattern(&shared.params.morse_text.lock().unwrap().clone());
    let mut i = 0usize;

    while !is_stopped(stop) {
        let (min, max) = level_range(&shared.params);
        let (on, units) = pattern[i % pattern.len()];
        shared.emit(if on { max } else { min });

        let unit_ms = interval_from_speed(shared.params.speed.load(Ordering::Relaxed));
        for _ in 0..units {
            capped_sleep(unit_ms);
            if is_stopped(stop) {
                return;
            }
        }
        i += 1;
    }
}