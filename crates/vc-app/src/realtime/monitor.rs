use super::*;

pub(super) struct MonitorWriter {
    producer: rtrb::Producer<(u64, f32)>,
    epoch: Arc<AtomicU64>,
    previous_epoch: u64,
    source_rate: u32,
    output_rate: u32,
    resampler: dsp::StreamingResampleMono,
    prepared: Vec<f32>,
    telemetry: Arc<Telemetry>,
}

pub(super) fn open(
    host: AudioHost,
    device: &str,
    source_rate: u32,
    source_chunk: usize,
    epoch: Arc<AtomicU64>,
    telemetry: Arc<Telemetry>,
) -> Result<(AudioStream, MonitorWriter)> {
    if !audio::cpal_output_names(host)?
        .iter()
        .any(|name| name == device)
    {
        bail!("Monitor device is unavailable: {device}. Select connected headphones.");
    }
    let (producer, mut consumer) = RingBuffer::new(65_536);
    let callback_epoch = Arc::clone(&epoch);
    let callback_telemetry = Arc::clone(&telemetry);
    let queue_limit = Arc::new(AtomicU64::new(65_536));
    let callback_limit = Arc::clone(&queue_limit);
    let (stream, output_rate) = audio::test_output(host, Some(device), false, 0, move |samples| {
        let dropped = trim_backlog(
            &mut consumer,
            callback_limit.load(Ordering::Relaxed) as usize,
        );
        callback_telemetry
            .monitor_dropped_samples
            .fetch_add(dropped as u64, Ordering::Relaxed);
        let epoch = callback_epoch.load(Ordering::Acquire);
        let played = fill_output(&mut consumer, epoch, samples);
        callback_telemetry
            .monitor_played_samples
            .fetch_add(played as u64, Ordering::Relaxed);
        if epoch & 1 != 0 {
            callback_telemetry
                .monitor_missing_samples
                .fetch_add((samples.len() - played) as u64, Ordering::Relaxed);
        }
    })?;
    queue_limit.store(
        (2 * source_chunk as u64 * output_rate as u64 / source_rate as u64
            + output_rate as u64 / 50)
            .min(65_536),
        Ordering::Relaxed,
    );
    Ok((
        stream,
        MonitorWriter {
            telemetry,
            producer,
            previous_epoch: epoch.load(Ordering::Acquire),
            epoch,
            source_rate,
            output_rate,
            resampler: dsp::StreamingResampleMono::new(source_rate as usize, output_rate as usize)?,
            prepared: Vec::with_capacity(
                source_chunk * output_rate as usize / source_rate as usize + 1024,
            ),
        },
    ))
}

fn trim_backlog(consumer: &mut rtrb::Consumer<(u64, f32)>, limit: usize) -> usize {
    let excess = consumer.slots().saturating_sub(limit);
    for _ in 0..excess {
        let _ = consumer.pop();
    }
    excess
}

// Epoch-tagged samples prevent a quick off/on sequence from replaying old audio.
// No allocation, locks, resampling, or logging occurs in this callback helper.
fn fill_output(consumer: &mut rtrb::Consumer<(u64, f32)>, epoch: u64, output: &mut [f32]) -> usize {
    output.fill(0.0);
    if epoch & 1 == 0 {
        for _ in 0..consumer.slots() {
            let _ = consumer.pop();
        }
        return 0;
    }
    let mut played = 0;
    for sample in output {
        while let Ok((sample_epoch, value)) = consumer.pop() {
            if sample_epoch == epoch {
                *sample = value;
                played += 1;
                break;
            }
        }
    }
    played
}

impl MonitorWriter {
    pub(super) fn push(&mut self, samples: &[f32]) -> Result<()> {
        let epoch = self.epoch.load(Ordering::Acquire);
        if epoch != self.previous_epoch {
            self.resampler = dsp::StreamingResampleMono::new(
                self.source_rate as usize,
                self.output_rate as usize,
            )?;
            self.previous_epoch = epoch;
        }
        if epoch & 1 == 0 {
            return Ok(());
        }
        self.prepared.clear();
        self.resampler.process_into(samples, &mut self.prepared)?;
        for &sample in &self.prepared {
            if self.producer.push((epoch, sample)).is_err() {
                self.telemetry
                    .monitor_dropped_samples
                    .fetch_add(1, Ordering::Relaxed);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backlog_trimming_keeps_latest_audio() {
        let (mut producer, mut consumer) = RingBuffer::new(8);
        for value in [0.1, 0.2, 0.3, 0.4] {
            producer.push((1, value)).unwrap();
        }
        assert_eq!(trim_backlog(&mut consumer, 2), 2);
        let mut output = [0.0; 2];
        assert_eq!(fill_output(&mut consumer, 1, &mut output), 2);
        assert_eq!(output, [0.3, 0.4]);
    }

    #[test]
    fn mute_discards_queued_audio() {
        let (mut producer, mut consumer) = RingBuffer::new(8);
        producer.push((1, 0.5)).unwrap();
        let mut output = [1.0; 2];
        assert_eq!(fill_output(&mut consumer, 2, &mut output), 0);
        assert_eq!(output, [0.0; 2]);
        assert_eq!(consumer.slots(), 0);
    }

    #[test]
    fn reenable_plays_only_current_epoch() {
        let (mut producer, mut consumer) = RingBuffer::new(8);
        producer.push((1, 0.5)).unwrap();
        producer.push((5, 0.25)).unwrap();
        let mut output = [0.0; 2];
        assert_eq!(fill_output(&mut consumer, 5, &mut output), 1);
        assert_eq!(output, [0.25, 0.0]);
    }
}
