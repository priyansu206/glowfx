//! Audio-reactive effect: capture loopback (WASAPI loopback / PipeWire monitor)
//! or fall back to the default input, compute RMS, quantize to {0,1,2}.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Data, Device, SampleFormat, Stream, SupportedStreamConfig,
};

use super::{capped_sleep, is_stopped, EffectShared};

pub fn run(shared: Arc<EffectShared>, stop: Arc<AtomicBool>) {
    let rms = Arc::new(AtomicU32::new(0.0f32.to_bits()));
    let mut prev = 0.0f32;

    let cap_shared = Arc::clone(&shared);
    let cap_stop = Arc::clone(&stop);
    let cap_rms = Arc::clone(&rms);
    let capture: JoinHandle<()> =
        thread::spawn(move || capture_thread(&cap_shared, &cap_stop, &cap_rms));

    while !is_stopped(&stop) {
        let v = rms_load(&rms);
        let sens = shared.params.sensitivity.load(Ordering::Relaxed) as f32 / 100.0;

        let level = if shared.params.audio_beat.load(Ordering::Relaxed) {
            // Beat sync: flash on rising transients (kick/hit), else rest.
            let rise = (v - prev).max(0.0);
            prev = prev * 0.8 + v * 0.2;
            let floor = (0.05 + (1.0 - sens) * 0.10).min(0.9);
            if rise > 0.03 && v > floor {
                2
            } else if v >= floor {
                1
            } else {
                0
            }
        } else {
            // Higher sensitivity -> lower thresholds.
            let low = (0.02 + (1.0 - sens) * 0.30).min(0.9);
            let high = (0.10 + (1.0 - sens) * 0.40).max(low + 0.05);
            if v >= high {
                2
            } else if v >= low {
                1
            } else {
                0
            }
        };
        shared.emit(level);

        // Throttled: audio never writes faster than 10 Hz regardless of capture rate.
        capped_sleep(120);
    }

    let _ = capture.join();
}

fn rms_load(a: &Arc<AtomicU32>) -> f32 {
    f32::from_bits(a.load(Ordering::Relaxed))
}

fn rms_store(a: &Arc<AtomicU32>, v: f32) {
    a.store(v.to_bits(), Ordering::Relaxed);
}

fn capture_thread(shared: &EffectShared, stop: &AtomicBool, rms: &Arc<AtomicU32>) {
    if is_stopped(stop) {
        return;
    }
    let stream = match build_host_capture(rms) {
        Ok(s) => s,
        Err(e) => {
            shared.report_error(e);
            return;
        }
    };
    if let Err(e) = stream.play() {
        shared.report_error(format!("audio stream failed to start: {e}"));
        return;
    }
    // Keep the stream alive (cpal callbacks run on their own thread) until stop.
    while !is_stopped(stop) {
        thread::sleep(Duration::from_millis(150));
    }
    drop(stream);
}

/// Prefer loopback on the default output device; fall back to the default input.
fn build_host_capture(rms: &Arc<AtomicU32>) -> Result<Stream, String> {
    let host = cpal::default_host();

    let mut attempt: Option<(Device, SupportedStreamConfig)> = None;
    if let Some(dev) = host.default_output_device() {
        if let Ok(cfg) = dev.default_input_config() {
            attempt = Some((dev, cfg));
        }
    }
    if attempt.is_none() {
        if let Some(mic) = host.default_input_device() {
            if let Ok(cfg) = mic.default_input_config() {
                attempt = Some((mic, cfg));
            }
        }
    }

    let (device, config) =
        attempt.ok_or_else(|| "No audio capture device found (loopback or mic)".to_string())?;
    let stream_config = config.config();
    let format = config.sample_format();

    let err_fn = |e| eprintln!("[glowfx] audio stream error: {e}");

    let rms = Arc::clone(rms);
    let stream = device.build_input_stream_raw(
        &stream_config,
        format,
        move |data: &Data, _: &cpal::InputCallbackInfo| {
            feed_rms(data, &rms);
        },
        err_fn,
        None,
    );

    match stream {
        Ok(s) => Ok(s),
        Err(e) => Err(format!("failed to open audio capture stream: {e}")),
    }
}

/// Interpret whatever sample format the host delivered and fold it into the RMS.
fn feed_rms(data: &Data, rms: &Arc<AtomicU32>) {
    match data.sample_format() {
        SampleFormat::F32 => rms_fold(data.as_slice::<f32>().map(|b| b.iter().copied()), rms),
        SampleFormat::F64 => rms_fold(
            data.as_slice::<f64>().map(|b| b.iter().map(|&s| s as f32)),
            rms,
        ),
        SampleFormat::I8 => rms_fold(
            data.as_slice::<i8>().map(|b| b.iter().map(|&s| s as f32 / 128.0)),
            rms,
        ),
        SampleFormat::I16 => rms_fold(
            data.as_slice::<i16>().map(|b| b.iter().map(|&s| s as f32 / 32_768.0)),
            rms,
        ),
        SampleFormat::I32 => rms_fold(
            data.as_slice::<i32>()
                .map(|b| b.iter().map(|&s| s as f32 / 2_147_483_648.0)),
            rms,
        ),
        SampleFormat::I64 => rms_fold(
            data.as_slice::<i64>().map(|b| b.iter().map(|&s| s as f32)),
            rms,
        ),
        SampleFormat::U8 => rms_fold(
            data.as_slice::<u8>().map(|b| b.iter().map(|&s| (s as f32 - 128.0) / 128.0)),
            rms,
        ),
        SampleFormat::U16 => rms_fold(
            data.as_slice::<u16>()
                .map(|b| b.iter().map(|&s| (s as f32 - 32_768.0) / 32_768.0)),
            rms,
        ),
        SampleFormat::U32 => rms_fold(
            data.as_slice::<u32>()
                .map(|b| b.iter().map(|&s| (s as f32 / 2_147_483_648.0) - 1.0)),
            rms,
        ),
        SampleFormat::U64 => rms_fold(
            data.as_slice::<u64>().map(|b| b.iter().map(|&s| s as f32)),
            rms,
        ),
        _ => {}
    }
}

fn rms_fold<I>(samples: Option<I>, rms: &Arc<AtomicU32>)
where
    I: IntoIterator<Item = f32>,
{
    let Some(samples) = samples else {
        return;
    };
    let mut sum = 0.0f32;
    let mut n = 0usize;
    for v in samples {
        sum += v * v;
        n += 1;
    }
    let new = if n > 0 { (sum / n as f32).sqrt().min(1.0) } else { 0.0 };
    let old = rms_load(rms);
    rms_store(rms, old * 0.7 + new * 0.3);
}