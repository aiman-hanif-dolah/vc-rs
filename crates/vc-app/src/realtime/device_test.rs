//! Model-free diagnostics owned by the same controller as conversion. A test
//! request cannot contain model paths or a provider, so saved RVC configuration
//! can never accidentally trigger inference while checking a microphone.
use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TestOutput {
    #[default]
    Silent,
    Tone,
    Monitor,
}

#[derive(Clone, Debug)]
pub struct DeviceTestConfig {
    pub input_host: AudioHost,
    pub output_host: AudioHost,
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    pub input_exclusive: bool,
    pub output_exclusive: bool,
    pub buffer_ms: u32,
    pub rnnoise: bool,
    pub output: TestOutput,
}

#[derive(Clone, Debug, Default)]
pub struct DeviceTestSnapshot {
    pub active: bool,
    pub input_ready: bool,
    pub output: TestOutput,
    pub rms: f32,
    pub peak: f32,
    pub clipping: bool,
    pub input_error: Option<String>,
    pub output_error: Option<String>,
}

pub(super) struct DeviceTestSession {
    input: Option<AudioStream>,
    output: Option<AudioStream>,
    running: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

fn levels(samples: &[f32], gain: f32) -> (f32, f32) {
    let mut sum = 0.0;
    let mut peak = 0.0_f32;
    for &sample in samples {
        let value = sample * gain;
        sum += value * value;
        peak = peak.max(value.abs());
    }
    ((sum / samples.len().max(1) as f32).sqrt(), peak)
}

fn tone_sample(frame: u32, rate: u32, gain: f32) -> f32 {
    if frame >= rate {
        return 0.0;
    }
    let fade = (rate as f32 * 0.01).max(1.0);
    let envelope = (frame as f32 / fade)
        .min((rate - 1 - frame) as f32 / fade)
        .min(1.0);
    (std::f32::consts::TAU * 440.0 * frame as f32 / rate as f32).sin() * 0.1 * gain * envelope
}

impl DeviceTestSession {
    pub(super) fn start(
        config: DeviceTestConfig,
        live: Arc<AtomicLiveParams>,
        snapshot: Arc<Mutex<DeviceTestSnapshot>>,
    ) -> Result<Self> {
        let running = Arc::new(AtomicBool::new(true));
        let (mut capture, mut input_queue) = RingBuffer::<f32>::new(65_536);
        let input_running = running.clone();
        let input = audio::test_input(
            config.input_host,
            config.input_device.as_deref(),
            config.input_exclusive,
            config.buffer_ms,
            move |samples| {
                if input_running.load(Ordering::Relaxed) {
                    for &sample in samples {
                        let _ = capture.push(sample);
                    }
                }
            },
        )
        .and_then(|(stream, rate)| {
            stream.play()?;
            Ok((stream, rate))
        });
        let (mut playback, mut output_queue) = RingBuffer::<f32>::new(65_536);
        let output_running = running.clone();
        let consumed = Arc::new(AtomicU32::new(0));
        let callback_consumed = consumed.clone();
        // Silent tests don't even open the output device. A missing microphone
        // likewise doesn't prevent an explicit output-only tone test.
        let output = if config.output != TestOutput::Silent {
            Some(
                audio::test_output(
                    config.output_host,
                    config.output_device.as_deref(),
                    config.output_exclusive,
                    config.buffer_ms,
                    move |samples| {
                        for sample in samples {
                            *sample = if output_running.load(Ordering::Relaxed) {
                                match output_queue.pop() {
                                    Ok(value) => {
                                        callback_consumed.fetch_add(1, Ordering::Relaxed);
                                        value
                                    }
                                    Err(_) => 0.0,
                                }
                            } else {
                                0.0
                            };
                        }
                    },
                )
                .and_then(|(stream, rate)| {
                    stream.play()?;
                    Ok((stream, rate))
                }),
            )
        } else {
            None
        };
        let input_error = input.as_ref().err().map(|e| format!("{e:#}"));
        let output_error = output
            .as_ref()
            .and_then(|r| r.as_ref().err())
            .map(|e| format!("{e:#}"));
        let (input, input_rate) = input.map(|(s, r)| (Some(s), r)).unwrap_or((None, 48_000));
        let (output, output_rate) = output
            .and_then(Result::ok)
            .map(|(s, r)| (Some(s), r))
            .unwrap_or((None, input_rate));
        let mode = if output.is_some() && (config.output != TestOutput::Monitor || input.is_some())
        {
            config.output
        } else {
            TestOutput::Silent
        };
        *snapshot.lock().unwrap() = DeviceTestSnapshot {
            active: true,
            input_ready: input.is_some(),
            output: mode,
            input_error,
            output_error,
            ..Default::default()
        };
        let worker_running = running.clone();
        let worker = thread::Builder::new()
            .name("vc-device-test".into())
            .spawn(move || {
                let result = (|| -> Result<()> {
                    let mut params = live.load();
                    // GTCRN is deliberately inaccessible here. The enum construction
                    // below selects no backend code; only Off/RNNoise can be built.
                    params.noise_gate_enabled = false;
                    let mut processor = PassthroughProcessor::new(
                        if config.rnnoise {
                            DenoiserMode::Rnnoise
                        } else {
                            DenoiserMode::Off
                        },
                        NoiseGateShaping::default(),
                        input_rate,
                        output_rate,
                        None,
                        #[cfg(feature = "gtcrn")]
                        vc_core::denoise::GtcrnBackend::OrtCpu,
                        &params,
                    )?;
                    let hop = (input_rate / 100).max(1) as usize;
                    let mut chunk = Vec::with_capacity(hop);
                    let mut prepared = Vec::with_capacity(output_rate as usize / 10);
                    let mut tone_frame = 0;
                    let mut clip_until = Instant::now();
                    let mut last_input = Instant::now();
                    while worker_running.load(Ordering::Relaxed) {
                        params = live.load();
                        params.noise_gate_enabled = false;
                        while chunk.len() < hop {
                            match input_queue.pop() {
                                Ok(s) => chunk.push(s),
                                Err(_) => break,
                            }
                        }
                        if chunk.len() == hop {
                            last_input = Instant::now();
                            let (rms, peak) = levels(&chunk, params.input_gain);
                            if peak >= 1.0 {
                                clip_until = Instant::now() + Duration::from_secs(1);
                            }
                            {
                                let mut state = snapshot.lock().unwrap();
                                state.rms = rms;
                                state.peak = peak;
                                state.clipping = Instant::now() < clip_until;
                            }
                            if mode == TestOutput::Monitor {
                                processor.process_chunk(&chunk, &params, &mut prepared)?;
                                for &sample in &prepared {
                                    let _ = playback.push(sample);
                                }
                            }
                            chunk.clear();
                        }
                        if last_input.elapsed() > Duration::from_millis(250) {
                            let mut state = snapshot.lock().unwrap();
                            state.rms = 0.0;
                            state.peak = 0.0;
                            state.clipping = Instant::now() < clip_until;
                        }
                        if mode == TestOutput::Tone {
                            // Generate on this worker, never the output callback.
                            while tone_frame < output_rate
                                && playback.slots()
                                    > 65_536 - (output_rate as usize / 50).min(32_768)
                            {
                                playback
                                    .push(tone_sample(tone_frame, output_rate, params.output_gain))
                                    .unwrap();
                                tone_frame += 1;
                            }
                            if consumed.load(Ordering::Relaxed) >= output_rate {
                                snapshot.lock().unwrap().output = TestOutput::Silent;
                            }
                        }
                        thread::sleep(Duration::from_millis(2));
                    }
                    Ok(())
                })();
                if let Err(error) = result {
                    worker_running.store(false, Ordering::Relaxed);
                    snapshot.lock().unwrap().output_error = Some(format!("{error:#}"));
                }
            })?;
        Ok(Self {
            input,
            output,
            running,
            worker: Some(worker),
        })
    }

    pub(super) fn failed(&self) -> bool {
        !self.running.load(Ordering::Relaxed)
            || self.input.as_ref().is_some_and(AudioStream::has_error)
            || self.output.as_ref().is_some_and(AudioStream::has_error)
    }
}

impl Drop for DeviceTestSession {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        drop(self.input.take());
        drop(self.output.take());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        // Rings are owned by this session and discarded. No queued test audio
        // can leak into a new device or conversion session after Stop completes.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gain_levels_are_measured_before_clipping() {
        let (rms, peak) = levels(&[0.5, -0.5], 3.0);
        assert_eq!((rms, peak), (1.5, 1.5));
        assert_eq!(levels(&[0.0; 8], 12.0), (0.0, 0.0));
    }
    #[test]
    fn tone_is_bounded_fades_and_ends_at_one_second() {
        for rate in [44_100, 48_000, 96_000] {
            assert_eq!(tone_sample(0, rate, 1.0), 0.0);
            assert_eq!(tone_sample(rate - 1, rate, 1.0), 0.0);
            assert_eq!(tone_sample(rate, rate, 1.0), 0.0);
            let peak = (0..rate)
                .map(|i| tone_sample(i, rate, 0.3).abs())
                .fold(0.0, f32::max);
            assert!(peak <= 0.03001 && peak > 0.029);
        }
    }

    #[cfg(feature = "rnnoise")]
    #[test]
    fn monitor_reuses_denoising_and_resampling_for_different_device_rates() {
        for (input_rate, output_rate) in [(44_100, 48_000), (48_000, 44_100)] {
            let live = LiveParams {
                input_gain: 2.0,
                output_gain: 0.3,
                ..Default::default()
            };
            let mut processor = PassthroughProcessor::new(
                DenoiserMode::Rnnoise,
                NoiseGateShaping::default(),
                input_rate,
                output_rate,
                None,
                #[cfg(feature = "gtcrn")]
                vc_core::denoise::GtcrnBackend::OrtCpu,
                &live,
            )
            .unwrap();
            let input: Vec<f32> = (0..input_rate / 100)
                .map(|i| (std::f32::consts::TAU * 220.0 * i as f32 / input_rate as f32).sin() * 0.2)
                .collect();
            let mut output = Vec::new();
            let mut count = 0;
            for _ in 0..100 {
                processor.process_chunk(&input, &live, &mut output).unwrap();
                assert!(output.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
                count += output.len();
            }
            assert!(count > output_rate as usize * 9 / 10);
        }
    }

    #[test]
    #[ignore = "Opens the default microphone and plays a quiet one-second test tone; no recordings or inference"]
    fn live_device_test_independent_endpoints_and_stop() {
        let controller = EngineController::new(LiveParams {
            output_gain: 0.3,
            ..Default::default()
        });
        let mut config = DeviceTestConfig {
            input_host: AudioHost::default(),
            output_host: AudioHost::default(),
            input_device: None,
            output_device: Some("vc-rs deliberately unavailable test output".into()),
            input_exclusive: false,
            output_exclusive: false,
            buffer_ms: 0,
            rnnoise: false,
            output: TestOutput::Tone,
        };
        controller.start_device_test(config.clone()).unwrap();
        let wait = |predicate: &dyn Fn(&DeviceTestSnapshot) -> bool| {
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                let state = controller.device_test_snapshot();
                if predicate(&state) {
                    return state;
                }
                assert!(Instant::now() < deadline, "device test timeout: {state:?}");
                thread::sleep(Duration::from_millis(50));
            }
        };
        let state = wait(&|s| s.active && s.output_error.is_some());
        assert!(
            state.input_ready,
            "default microphone unavailable: {state:?}"
        );
        assert_eq!(state.output, TestOutput::Silent);
        controller.stop().unwrap();
        wait(&|s| !s.active);
        config.input_device = Some("vc-rs deliberately unavailable test microphone".into());
        config.output_device = None;
        controller.start_device_test(config).unwrap();
        let state = wait(&|s| s.active && s.input_error.is_some());
        assert!(
            state.output_error.is_none(),
            "default output unavailable: {state:?}"
        );
        wait(&|s| s.active && s.output == TestOutput::Silent);
        controller.stop().unwrap();
        let stopped = wait(&|s| !s.active);
        assert_eq!((stopped.rms, stopped.peak), (0.0, 0.0));
        assert_eq!(controller.snapshot().0.state, EngineState::Stopped);
    }
}
