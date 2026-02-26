use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::time::Instant;

use crate::stt::{SttConfig, SttMode};
use crate::transcribe::transcribe_with_cached_whisper;

/// Spawn a persistent audio thread that builds and immediately starts the cpal
/// input stream.  The stream runs for the entire app lifetime — the callback
/// checks `is_recording` atomically and discards samples when false.
pub fn spawn_audio_thread(
    buffer: Arc<Mutex<Vec<f32>>>,
    is_recording: Arc<AtomicBool>,
) -> Result<u32, String> {
    let (init_tx, init_rx) = mpsc::channel::<Result<u32, String>>();

    let buf_for_thread = Arc::clone(&buffer);
    let rec_for_thread = Arc::clone(&is_recording);

    std::thread::spawn(move || {
        let host = cpal::default_host();

        let device = match host.default_input_device() {
            Some(d) => d,
            None => {
                let _ = init_tx.send(Err("找不到麥克風裝置".to_string()));
                return;
            }
        };

        let config = match device.default_input_config() {
            Ok(c) => c,
            Err(e) => {
                let _ = init_tx.send(Err(format!("無法取得輸入設定: {}", e)));
                return;
            }
        };

        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        let stream = {
            let buf = Arc::clone(&buf_for_thread);
            let rec = Arc::clone(&rec_for_thread);
            match config.sample_format() {
                cpal::SampleFormat::F32 => device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if !rec.load(Ordering::Relaxed) {
                            return;
                        }
                        let mut buf = buf.lock().unwrap();
                        if channels == 1 {
                            buf.extend_from_slice(data);
                        } else {
                            for chunk in data.chunks(channels) {
                                buf.push(chunk.iter().sum::<f32>() / channels as f32);
                            }
                        }
                    },
                    |err| eprintln!("[Sumi] audio stream error: {}", err),
                    None,
                ),
                cpal::SampleFormat::I16 => {
                    let buf = Arc::clone(&buf_for_thread);
                    let rec = Arc::clone(&rec_for_thread);
                    device.build_input_stream(
                        &config.into(),
                        move |data: &[i16], _: &cpal::InputCallbackInfo| {
                            if !rec.load(Ordering::Relaxed) {
                                return;
                            }
                            let mut buf = buf.lock().unwrap();
                            if channels == 1 {
                                buf.extend(data.iter().map(|&s| s as f32 / i16::MAX as f32));
                            } else {
                                for chunk in data.chunks(channels) {
                                    buf.push(
                                        chunk
                                            .iter()
                                            .map(|&s| s as f32 / i16::MAX as f32)
                                            .sum::<f32>()
                                            / channels as f32,
                                    );
                                }
                            }
                        },
                        |err| eprintln!("[Sumi] audio stream error: {}", err),
                        None,
                    )
                }
                other => {
                    let _ = init_tx.send(Err(format!("不支援的音訊格式: {:?}", other)));
                    return;
                }
            }
        };

        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                let _ = init_tx.send(Err(format!("無法建立錄音串流: {}", e)));
                return;
            }
        };

        if let Err(e) = stream.play() {
            let _ = init_tx.send(Err(format!("無法啟動錄音串流: {}", e)));
            return;
        }

        println!(
            "[Sumi] Audio stream always-on: {} Hz, {} ch",
            sample_rate, channels
        );
        let _ = init_tx.send(Ok(sample_rate));

        // Park the thread forever to keep `stream` alive.
        loop {
            std::thread::park();
        }
    });

    let sample_rate = init_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| "音訊執行緒初始化逾時".to_string())??;

    Ok(sample_rate)
}

/// Attempt to reconnect the microphone when `mic_available` is false.
pub fn try_reconnect_audio(
    mic_available: &AtomicBool,
    sample_rate: &Mutex<Option<u32>>,
    buffer: &Arc<Mutex<Vec<f32>>>,
    is_recording: &Arc<AtomicBool>,
) -> Result<(), String> {
    if mic_available.load(Ordering::SeqCst) {
        return Ok(());
    }
    let sr = spawn_audio_thread(Arc::clone(buffer), Arc::clone(is_recording))?;
    *sample_rate.lock().map_err(|e| e.to_string())? = Some(sr);
    mic_available.store(true, Ordering::SeqCst);
    println!("[Sumi] Microphone reconnected: {} Hz", sr);
    Ok(())
}

/// Start recording — truly instant because the audio stream is always running.
pub fn do_start_recording(
    is_recording: &AtomicBool,
    mic_available: &AtomicBool,
    sample_rate: &Mutex<Option<u32>>,
    buffer: &Arc<Mutex<Vec<f32>>>,
    is_recording_arc: &Arc<AtomicBool>,
) -> Result<(), String> {
    if !mic_available.load(Ordering::SeqCst) {
        try_reconnect_audio(mic_available, sample_rate, buffer, is_recording_arc)?;
    }

    if is_recording.load(Ordering::SeqCst) {
        return Err("已在錄音中".to_string());
    }

    {
        let mut buf = buffer.lock().map_err(|e| e.to_string())?;
        buf.clear();
    }

    is_recording.store(true, Ordering::SeqCst);

    Ok(())
}

/// Stop recording, transcribe, and return the text + 16 kHz samples for history.
pub fn do_stop_recording(
    is_recording: &AtomicBool,
    sample_rate_mutex: &Mutex<Option<u32>>,
    buffer: &Arc<Mutex<Vec<f32>>>,
    whisper_ctx: &Mutex<Option<whisper_rs::WhisperContext>>,
    http_client: &reqwest::blocking::Client,
    stt_config: &SttConfig,
) -> Result<(String, Vec<f32>), String> {
    let sample_rate = sample_rate_mutex
        .lock()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No microphone available".to_string())?;

    if is_recording
        .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("目前未在錄音".to_string());
    }

    let samples: Vec<f32> = {
        let buf = buffer.lock().map_err(|e| e.to_string())?;
        buf.clone()
    };

    if samples.is_empty() {
        return Err("沒有錄到任何聲音".to_string());
    }

    println!(
        "[Sumi] [timing] recording: {:.2}s ({} samples @ {} Hz)",
        samples.len() as f64 / sample_rate as f64,
        samples.len(),
        sample_rate,
    );

    let t0 = Instant::now();
    let mut samples_16k = if sample_rate != 16000 {
        let resampled = resample(&samples, sample_rate, 16000);
        println!("[Sumi] [timing] resample {} Hz → 16 kHz: {:.0?}", sample_rate, t0.elapsed());
        resampled
    } else {
        samples
    };

    // Strip leading silence
    const SILENCE_RMS_THRESHOLD: f32 = 0.01;
    const WINDOW: usize = 160;
    const LOOKBACK: usize = 1600;

    let speech_onset = samples_16k
        .windows(WINDOW)
        .position(|w| {
            let rms = (w.iter().map(|&s| s * s).sum::<f32>() / WINDOW as f32).sqrt();
            rms > SILENCE_RMS_THRESHOLD
        })
        .unwrap_or(0);

    let trim_start = speech_onset.saturating_sub(LOOKBACK);
    if trim_start > 0 {
        println!(
            "[Sumi] Trimmed {:.0} ms of leading silence (onset at {:.0} ms)",
            trim_start as f64 / 16.0,
            speech_onset as f64 / 16.0
        );
        samples_16k = samples_16k[trim_start..].to_vec();
    }

    // Strip trailing silence
    if samples_16k.len() > WINDOW {
        let total = samples_16k.len();
        let last_speech = samples_16k
            .windows(WINDOW)
            .rposition(|w| {
                let rms = (w.iter().map(|&s| s * s).sum::<f32>() / WINDOW as f32).sqrt();
                rms > SILENCE_RMS_THRESHOLD
            })
            .map(|pos| pos + WINDOW)
            .unwrap_or(total);

        let trim_end = (last_speech + LOOKBACK).min(total);
        if trim_end < total {
            println!(
                "[Sumi] Trimmed {:.0} ms of trailing silence",
                (total - trim_end) as f64 / 16.0
            );
            samples_16k.truncate(trim_end);
        }
    }

    let stt_start = Instant::now();
    let text = match stt_config.mode {
        SttMode::Local => {
            let result = transcribe_with_cached_whisper(whisper_ctx, &samples_16k)?;
            println!("[Sumi] [timing] STT (local whisper): {:.0?}", stt_start.elapsed());
            result
        }
        SttMode::Cloud => {
            let result = crate::stt::run_cloud_stt(&stt_config.cloud, &samples_16k, http_client)?;
            println!("[Sumi] [timing] STT (cloud {}): {:.0?}", stt_config.cloud.provider.as_key(), stt_start.elapsed());
            result
        }
    };

    if text.is_empty() {
        Err("no_speech".to_string())
    } else {
        Ok((text, samples_16k))
    }
}

/// Simple linear interpolation resampler.
pub fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (samples.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);
    for i in 0..output_len {
        let src_idx = i as f64 * ratio;
        let idx = src_idx as usize;
        let frac = src_idx - idx as f64;
        let sample = if idx + 1 < samples.len() {
            samples[idx] as f64 * (1.0 - frac) + samples[idx + 1] as f64 * frac
        } else {
            samples[idx.min(samples.len() - 1)] as f64
        };
        output.push(sample as f32);
    }
    output
}
