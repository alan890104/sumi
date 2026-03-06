//! Measures CoreAudio mic stream initialization latency.
//!
//! Each iteration: cold-start a CPAL input stream → stream.play() → ready.
//! Used to determine whether on-demand mic open/close is perceptible.
//!
//! Run with:
//!   cargo run --bin mic_latency

use std::sync::mpsc;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// Open a mic stream from scratch, return elapsed time until ready.
/// The stream is kept alive for `hold_ms` before being dropped so CoreAudio
/// has time to settle before the next iteration.
fn measure_one(hold_ms: u64) -> Result<Duration, String> {
    let (tx, rx) = mpsc::channel::<Result<(), String>>();

    let t0 = Instant::now();

    std::thread::spawn(move || {
        let host = cpal::default_host();

        let device = match host.default_input_device() {
            Some(d) => d,
            None => {
                let _ = tx.send(Err("No default input device".into()));
                return;
            }
        };

        let config = match device.default_input_config() {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(Err(format!("default_input_config: {e}")));
                return;
            }
        };

        let stream_result = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                |_: &[f32], _| {},
                |e| eprintln!("stream error: {e}"),
                None,
            ),
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                |_: &[i16], _| {},
                |e| eprintln!("stream error: {e}"),
                None,
            ),
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config.into(),
                |_: &[u16], _| {},
                |e| eprintln!("stream error: {e}"),
                None,
            ),
            other => {
                let _ = tx.send(Err(format!("Unsupported sample format: {other:?}")));
                return;
            }
        };

        let stream = match stream_result {
            Ok(s) => s,
            Err(e) => {
                let _ = tx.send(Err(format!("build_input_stream: {e}")));
                return;
            }
        };

        if let Err(e) = stream.play() {
            let _ = tx.send(Err(format!("stream.play: {e}")));
            return;
        }

        // Signal ready — t0.elapsed() captured by caller after this.
        let _ = tx.send(Ok(()));

        // Hold the stream open briefly so CoreAudio can fully settle,
        // then drop it (stream closes when this thread exits).
        std::thread::sleep(Duration::from_millis(hold_ms));
    });

    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(())) => Ok(t0.elapsed()),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("Timeout waiting for stream init".into()),
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let idx = ((sorted.len() as f64 - 1.0) * p / 100.0).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn main() {
    const ITERATIONS: usize = 10;
    // Hold each stream open this long before closing, to let CoreAudio
    // fully release the device before the next open.
    const HOLD_MS: u64 = 300;
    // Gap between iterations.
    const GAP_MS: u64 = 200;

    println!("CoreAudio mic stream init latency test");
    println!("  Iterations : {ITERATIONS}");
    println!("  Stream hold: {HOLD_MS} ms per run (then closed)");
    println!("  Gap        : {GAP_MS} ms between runs");
    println!();

    let mut samples: Vec<f64> = Vec::with_capacity(ITERATIONS);

    for i in 0..ITERATIONS {
        if i > 0 {
            std::thread::sleep(Duration::from_millis(GAP_MS));
        }

        match measure_one(HOLD_MS) {
            Ok(elapsed) => {
                let ms = elapsed.as_secs_f64() * 1000.0;
                println!("  Run {:2}:  {:6.1} ms", i + 1, ms);
                samples.push(ms);
            }
            Err(e) => {
                println!("  Run {:2}:  ERROR — {e}", i + 1);
            }
        }
    }

    if samples.is_empty() {
        eprintln!("\nNo successful measurements — check mic permissions.");
        std::process::exit(1);
    }

    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mean = samples.iter().sum::<f64>() / samples.len() as f64;

    println!();
    println!("── Statistics ({} successful runs) ──────────────", samples.len());
    println!("  min  : {:6.1} ms", samples[0]);
    println!("  p50  : {:6.1} ms", percentile(&samples, 50.0));
    println!("  p95  : {:6.1} ms", percentile(&samples, 95.0));
    println!("  max  : {:6.1} ms", samples[samples.len() - 1]);
    println!("  mean : {:6.1} ms", mean);
    println!("─────────────────────────────────────────────────");
    println!();

    let verdict = percentile(&samples, 95.0);
    if verdict < 50.0 {
        println!("Verdict: p95 < 50 ms — on-demand open is imperceptible.");
    } else if verdict < 150.0 {
        println!("Verdict: p95 {verdict:.0} ms — marginal; overlay 'preparing' state can absorb it.");
    } else {
        println!("Verdict: p95 {verdict:.0} ms — noticeable delay; idle-timeout approach is safer.");
    }
}
