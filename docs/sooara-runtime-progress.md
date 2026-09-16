# Sooara runtime progress

## Scope and completion status

Personal Windows installation with noise reduction and natural voice conversion
is still in progress. A successful build or silent-microphone run is not proof
of perceived quality, Discord reception, or production readiness.

## Observed on 2026-09-16

- Windows ML GUI release build completed with `windowsml,rnnoise,gtcrn`.
- A 20-second model-free GTCRN run with 20 ms chunks reached Running and
  reported 901 processed chunks at its final logged snapshot. Observed processing
  was approximately 1.4–2.7 ms per chunk. Five startup underruns occurred;
  the counter then stayed constant. No input overruns or output drops appeared.
- A DirectML RVC run with RNNoise and 100 ms chunks loaded the converted LJ
  model and processed audio. Initial inference took seconds and caused input
  overruns, output underruns, and dropped output. Later snapshots showed roughly
  50–79 ms inference with stable error counters. This is not a passing latency
  result: nominal content delay alone was 169.812 ms, excluding device, queue,
  and chunk-accumulation delay.
- These runs used the existing virtual cable. They did not verify the receiving
  application or independent headphone monitoring. No old driver was removed.

## Local model candidate, not a redistributable asset

Source: `Coolwowsocoolwow/LJ_Dataset_Speaker` on Hugging Face, revision
`45f4aaac5c304d77f6d8536a292bda8a0a192f74`.
The model card contains only an `openrail` license tag. Detailed licensing and
provenance review is outstanding before including it in any starter pack.

- Archive: `LJ_Speech_Speaker_v2.zip`
- Archive SHA256: `87b8dbfdeb8bb3301d5a734e664eace7113c6ce95d5ed597986dd06b1ca3c7cf`
- Checkpoint: `LJ_Speech_Speaker_e100_s4800.pth`
- Streaming ONNX SHA256: `f5165d11c9f6ced4c67278c5b1b785d745e772781cc28632a9fd21d118df36c0`

Weights remain ignored local assets. Conversion used the shared Rust converter.

## Next acceptance work

### Bundled one-shot soundboard (2026-09-17)

Ten recorded Kenney Fighter voiceover clips are included under
`resources/soundboard`, with their original CC0 license and source provenance.
Every clip was decoded to mono 48 kHz PCM16 WAV at 0.5 gain for mixing headroom;
all ten were checked with FFprobe and have durations between 0.93 and 1.87 s.
These are game-announcer clips, not replacements falsely labeled as airhorns or
other meme sounds. Additional clip categories and custom imports remain work.

The GUI renders a compact responsive grid, up to ten columns. Each click starts
one-shot playback through the main output plus selected monitor headphones;
another click replaces the current sound, and Stop sounds stops both. It reuses
the asynchronous file player without restarting voice conversion. The main route
uses the active engine output when running. The installer now copies the audio
folder and license alongside the app.

A standalone invocation of the shared soundboard service played `fight.wav` to
the cable and headphones, then returned to idle. A separate approximately
16-second capture of the cable microphone contained one matching clip (normalized
correlation 0.9897 after analysis resampling), with no second comparable match
(best non-overlapping correlation 0.0233). This verifies one-shot delivery to the
receiving endpoint, not Discord reception or simultaneous microphone mixing.
The GUI build passed. A duplicate-route unit test was added, not run.

### In-app recording playback (2026-09-17)

The library now asynchronously indexes the preserved recordings directory and
renders a compact, virtualized list with direct Play buttons and measured WAV
durations. Refresh does not decode recordings on the UI thread. A read-only run
of the shared library scanner returned all six existing files: three 20.64-second
takes and three 1.97-second takes. No recording was moved or modified.

The normal GUI now has a Recording playback section with Choose recording,
Play once, and Stop playback. Its picker opens the preserved recording folder;
WAV decoding and playback use a separate `vc-app::AudioFilePlayer` worker.
Playback uses the explicitly selected monitor headphones, not the main output,
and does not stop live conversion. No external media application is launched.
Stop invalidates callback playback immediately, including pending file loads.
Preview currently supports WAV only, up to ten minutes / 30 million samples.

The shared player completed a one-shot run of the existing short dry take on the
headphone device and returned to idle. GUI release builds passed. End-of-file
and silence-tail unit tests were added, not executed. The GUI buttons themselves
remain unverified interactively. Recording capture and one-shot soundboard mixing
remain separate unfinished work.

### Receiving virtual microphone check (2026-09-17)

A separate FFmpeg DirectShow capture opened the cable microphone endpoint while
Sooara converted the physical microphone with RVC and RNNoise. Both processes
exited successfully. The received capture contained 9.98 seconds of non-silent
audio, approximately -33.37 dBFS RMS and -12.99 dBFS peak after analysis downsampling.
The original capture is stereo 44.1 kHz; the engine debug output is mono 48 kHz.

After resampling both to 8 kHz and aligning one-second windows independently,
the received windows at 0, 2, 4, 6 and 8 seconds correlated with the engine's
processed output at 0.9985, 1.0000, 1.0000, 1.0000 and 1.0000 respectively.
This demonstrates that another application can receive the converted signal
through the current cable. It does not verify Discord's selected device,
transmission, or a remote listener.

The alignment offset changed by 10 ms between the second and fourth received
seconds; whole-capture correlation was only 0.570. Investigate this timing
discontinuity before claiming uninterrupted long-run routing. Alignment offsets
are recording-start differences, not an end-to-end latency measurement.
The 30-second engine run also reported an input-driver xrun and output underruns.
Private diagnostic WAVs remain ignored in `assets/sooara-cable-*-20260917.wav`.

### Independent processed monitoring

The shared standalone runtime now supports an optional monitor device in addition
to the main output. The GUI's Audio section contains a headphone selector and
live `Hear myself` toggle; the CLI accepts `--monitor <device>`. Selection requires
a Start/Restart, but toggling does not reload models. Monitoring is off after
launch. Queued samples carry a toggle epoch so mute/unmute cannot replay an old
queue. The worker resamples the processed output for the monitor device; callbacks
only drain a bounded queue. A missing monitor endpoint or selecting the main
output as the monitor is rejected.

Windows ML builds passed after this addition. A 10-second RNNoise run opened both
the cable and headphone streams. A 15-second RVC/RNNoise run at 80 ms chunks,
20 ms crossfade and 60 ms extra context reached 104 logged converted chunks;
no input-ring overrun or main-output drops occurred. It reported one input-driver
xrun and 16 output underruns including startup. These checks do not measure
headphone acoustics or Discord reception. Long-run clock-drift validation remains
outstanding.

Monitor callback-consumed, missing, and dropped sample counters are now exposed
in CLI diagnostics and GUI Backend Details. Monitor backlog is capped at two
converted chunks plus 20 ms (subject to ring capacity); excess oldest audio is
discarded and counted rather than accumulating seconds of delayed monitoring.
A 15-second RNNoise follow-up reached 655680 callback-consumed samples at its
last logged snapshot, zero monitor drops, and 1536 startup missing samples that
did not increase. One input-driver xrun occurred. This validates callback
delivery, not audible quality. The updated preview passed its installed doctor
check. Queue-trimming and counter assertions were added but not run as tests.

The latest side-by-side installed preview contains this implementation and passed
its installed runtime diagnostic. New queue/mute regression tests are written but
were not executed. Interactive GUI verification could not run because the
computer-use JavaScript runtime is unavailable in this session.

### Recorded-speech conversion

A preserved 20.64-second dry take was converted with the 80/20/60 ms settings.
The result has the same 990720 samples at 48 kHz, peak -11.13 dBFS and RMS
-34.32 dBFS, without clipping. This proves file production and signal headroom,
not human naturalness. The join report includes low-correlation seams requiring
listening review. Files remain ignored under `assets/sooara-lj-rnnoise-80ms.*`.
Nominal content delay is 114.812 ms, not end-to-end latency. A separate 20-second
live run accumulated two further underruns after startup, so this candidate has
not replaced GUI defaults.

### Startup preparation follow-up

The standalone runtime now performs one complete RVC chunk before starting
device streams, then resets pipeline and converter history. A repeated 20-second
DirectML run at the same 100 ms configuration reached 145 logged live chunks,
with zero input overruns and zero dropped output samples. Startup underruns
fell to 20 and stayed constant. Nominal content delay remained 169.812 ms;
warm-up fixes initial backlog, not steady-state latency.

A side-by-side local preview was installed with `scripts/install-sooara-local.ps1`.
The installed copy passed `doctor`, and its files were hash-checked against the
build. A per-user `Sooara Next` Start menu shortcut points to that version.
Support and candidate voice models were copied to the separate Sooara model
cache and checked against the source hashes. First-run GUI behavior and speech
quality are not yet verified. No microphone autostart was configured.

1. Verify warmed startup across restarts and model switches, including cancellation
   and live speech. Stop requests currently wait for preparation to finish.
2. Measure actual speech through the shared offline and live pipelines, then tune
   quality and latency rather than relying on silent input or nominal metrics.
3. Verify independent processed monitoring and virtual microphone reception.
4. Install side by side with required runtime files, preserve old recordings,
   and verify launch and device reconnection behavior.
5. Complete compact presets, one-shot soundboard audio, and in-app recording
   playback without blocking UI actions.

Test suites have not been run automatically, per the user's instructions.
