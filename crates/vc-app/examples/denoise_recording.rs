use std::fs::OpenOptions;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use vc_core::denoise::{GtcrnBackend, GtcrnConfig, GtcrnDenoiser, RnnoiseDenoiser};

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let mode = args.next().context(
        "usage: denoise_recording <rnnoise|gtcrn> <input.wav> <output.wav> [gtcrn-model-dir]",
    )?;
    let input = PathBuf::from(args.next().context("input WAV is required")?);
    let output = PathBuf::from(args.next().context("output WAV is required")?);
    if output.exists() {
        bail!("Refusing to overwrite {}", output.display());
    }
    let mut reader = hound::WavReader::open(&input)?;
    let spec = reader.spec();
    if spec.channels != 1
        || spec.bits_per_sample != 16
        || spec.sample_format != hound::SampleFormat::Int
    {
        bail!("This diagnostic accepts mono PCM16 WAV only");
    }
    if reader.duration() > spec.sample_rate.saturating_mul(600) {
        bail!("Recording must be ten minutes or shorter");
    }
    let samples = reader
        .samples::<i16>()
        .map(|sample| sample.map(|value| f32::from(value) / 32768.0))
        .collect::<Result<Vec<_>, _>>()?;
    if samples.is_empty() {
        bail!("Recording is empty");
    }
    let started = Instant::now();
    let processed = if mode == "rnnoise" {
        RnnoiseDenoiser::process_finite(&samples, spec.sample_rate)?
    } else if mode == "gtcrn" {
        let model_dir = PathBuf::from(args.next().context("GTCRN model directory is required")?);
        GtcrnDenoiser::new(
            GtcrnConfig {
                model_dir: &model_dir,
                backend: GtcrnBackend::OrtCpu,
            },
            spec.sample_rate,
        )?
        .process_finite(&samples)?
    } else {
        bail!("Unknown denoiser; choose rnnoise or gtcrn");
    };
    let elapsed = started.elapsed();
    if processed.len() != samples.len() || processed.iter().any(|sample| !sample.is_finite()) {
        bail!("Denoiser returned invalid output");
    }
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    let mut writer = hound::WavWriter::new(file, spec)?;
    for sample in &processed {
        writer.write_sample((sample.clamp(-1.0, 1.0) * 32767.0).round() as i16)?;
    }
    writer.finalize()?;
    println!(
        "frames={} rate={} elapsed_ms={} output={}",
        processed.len(),
        spec.sample_rate,
        elapsed.as_millis(),
        output.display()
    );
    Ok(())
}
