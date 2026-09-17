use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Result};

use crate::{audio, AudioHost};

#[derive(Clone, Debug, Default)]
pub struct PlaybackSnapshot {
    pub path: Option<PathBuf>,
    pub playing: bool,
    pub loading: bool,
    pub error: Option<String>,
}

enum Command {
    Play(PathBuf, AudioHost, String, usize),
    Stop,
    Shutdown,
}

pub struct AudioFilePlayer {
    generation: Arc<AtomicUsize>,
    tx: mpsc::SyncSender<Command>,
    state: Arc<Mutex<PlaybackSnapshot>>,
    worker: Option<JoinHandle<()>>,
}

impl Default for AudioFilePlayer {
    fn default() -> Self {
        let (tx, rx) = mpsc::sync_channel(8);
        let state = Arc::new(Mutex::new(PlaybackSnapshot::default()));
        let worker_state = Arc::clone(&state);
        let generation = Arc::new(AtomicUsize::new(0));
        let worker_generation = Arc::clone(&generation);
        let worker = thread::Builder::new()
            .name("sooara-recording-playback".into())
            .spawn(move || run(rx, worker_state, worker_generation))
            .expect("failed to spawn recording playback worker");
        Self {
            generation,
            tx,
            state,
            worker: Some(worker),
        }
    }
}

impl AudioFilePlayer {
    pub fn play(&self, path: PathBuf, host: AudioHost, device: String) -> Result<()> {
        if device.trim().is_empty() {
            bail!("Choose playback headphones first");
        }
        let generation = self
            .generation
            .fetch_add(1, Ordering::AcqRel)
            .wrapping_add(1);
        self.tx
            .try_send(Command::Play(path, host, device, generation))
            .map_err(|error| anyhow!("Playback request unavailable: {error}"))
    }

    pub fn stop(&self) -> Result<()> {
        self.generation.fetch_add(1, Ordering::AcqRel);
        self.tx
            .try_send(Command::Stop)
            .map_err(|error| anyhow!("Playback stop unavailable: {error}"))
    }

    pub fn snapshot(&self) -> PlaybackSnapshot {
        self.state
            .lock()
            .map(|state| state.clone())
            .unwrap_or_default()
    }
}

impl Drop for AudioFilePlayer {
    fn drop(&mut self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        let _ = self.tx.send(Command::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

struct Playback {
    generation: usize,
    stream: audio::AudioStream,
    position: Arc<AtomicUsize>,
    frames: usize,
    finished_at: Option<Instant>,
}

fn run(
    rx: mpsc::Receiver<Command>,
    state: Arc<Mutex<PlaybackSnapshot>>,
    generation: Arc<AtomicUsize>,
) {
    let mut playback: Option<Playback> = None;
    loop {
        match rx.recv_timeout(Duration::from_millis(25)) {
            Ok(Command::Play(path, host, device, requested)) => {
                if generation.load(Ordering::Acquire) != requested {
                    continue;
                }
                drop(playback.take());
                *state.lock().unwrap() = PlaybackSnapshot {
                    path: Some(path.clone()),
                    loading: true,
                    ..Default::default()
                };
                match open(&path, host, &device, Arc::clone(&generation), requested) {
                    Ok(session) => {
                        playback = Some(session);
                        let mut current = state.lock().unwrap();
                        current.loading = false;
                        current.playing = true;
                    }
                    Err(error) => {
                        let mut current = state.lock().unwrap();
                        current.loading = false;
                        current.error = Some(format!("{error:#}"));
                    }
                }
            }
            Ok(Command::Stop) => {
                drop(playback.take());
                let mut current = state.lock().unwrap();
                current.playing = false;
                current.loading = false;
            }
            Ok(Command::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if let Some(session) = playback.as_mut() {
            if session.generation != generation.load(Ordering::Acquire) {
                drop(playback.take());
                state.lock().unwrap().playing = false;
                continue;
            }
            if session.stream.has_error() {
                drop(playback.take());
                let mut current = state.lock().unwrap();
                current.playing = false;
                current.loading = false;
                current.error = Some(
                    "Playback device failed. Reconnect or select an output and play again.".into(),
                );
                continue;
            }
            session.stream.report_errors();
            if session.position.load(Ordering::Relaxed) >= session.frames {
                let finished = session.finished_at.get_or_insert_with(Instant::now);
                // Allow the device to drain the final callback before closing.
                if finished.elapsed() >= Duration::from_millis(100) {
                    drop(playback.take());
                    state.lock().unwrap().playing = false;
                }
            }
        }
    }
}

fn open(
    path: &Path,
    host: AudioHost,
    device: &str,
    generation: Arc<AtomicUsize>,
    requested: usize,
) -> Result<Playback> {
    let (samples, rate) = read_wav(path)?;
    let buffer = Arc::new(OnceLock::<Vec<f32>>::new());
    let callback_buffer = Arc::clone(&buffer);
    let position = Arc::new(AtomicUsize::new(0));
    let callback_position = Arc::clone(&position);
    let (stream, output_rate) = audio::test_output(host, Some(device), false, 0, move |output| {
        output.fill(0.0);
        if generation.load(Ordering::Acquire) != requested {
            return;
        }
        if let Some(samples) = callback_buffer.get() {
            let start = callback_position.load(Ordering::Relaxed);
            let end = fill_playback(samples, start, output);
            callback_position.store(end, Ordering::Relaxed);
        }
    })?;
    let prepared = vc_core::dsp::resample_mono(&samples, rate as usize, output_rate as usize)?;
    let frames = prepared.len();
    buffer
        .set(prepared)
        .map_err(|_| anyhow!("Playback buffer already initialized"))?;
    stream.play()?;
    Ok(Playback {
        generation: requested,
        stream,
        position,
        frames,
        finished_at: None,
    })
}

fn read_wav(path: &Path) -> Result<(Vec<f32>, u32)> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    if spec.channels == 0
        || spec.channels > 8
        || spec.sample_rate == 0
        || spec.sample_rate > 192_000
    {
        bail!("Unsupported recording channel count or sample rate");
    }
    if reader.duration() > spec.sample_rate.saturating_mul(600) || reader.len() > 30_000_000 {
        bail!("Recording preview is limited to ten minutes and 30 million samples");
    }
    let interleaved: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().collect::<Result<_, _>>()?,
        hound::SampleFormat::Int => {
            let scale = 2_f32.powi(i32::from(spec.bits_per_sample) - 1);
            reader
                .samples::<i32>()
                .map(|sample| sample.map(|sample| sample as f32 / scale))
                .collect::<Result<_, _>>()?
        }
    };
    if interleaved.is_empty() || interleaved.iter().any(|sample| !sample.is_finite()) {
        bail!("Recording is empty or contains invalid samples");
    }
    let mono = interleaved
        .chunks_exact(spec.channels as usize)
        .map(|frame| (frame.iter().sum::<f32>() / spec.channels as f32).clamp(-1.0, 1.0))
        .collect();
    Ok((mono, spec.sample_rate))
}

fn fill_playback(samples: &[f32], position: usize, output: &mut [f32]) -> usize {
    output.fill(0.0);
    let start = position.min(samples.len());
    let end = start.saturating_add(output.len()).min(samples.len());
    output[..end - start].copy_from_slice(&samples[start..end]);
    end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_finishes_once_and_fills_tail_with_silence() {
        let samples = [0.2, 0.4, 0.6];
        let mut output = [1.0; 2];
        let position = fill_playback(&samples, 0, &mut output);
        assert_eq!(output, [0.2, 0.4]);
        let position = fill_playback(&samples, position, &mut output);
        assert_eq!(position, 3);
        assert_eq!(output, [0.6, 0.0]);
        assert_eq!(fill_playback(&samples, position, &mut output), 3);
        assert_eq!(output, [0.0, 0.0]);
    }

    #[test]
    fn empty_playback_is_silent() {
        let mut output = [1.0; 2];
        assert_eq!(fill_playback(&[], 0, &mut output), 0);
        assert_eq!(output, [0.0; 2]);
    }
}
