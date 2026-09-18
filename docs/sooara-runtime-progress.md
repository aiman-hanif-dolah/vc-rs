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

### Latest installed process activated (2026-09-18)

The older running process was closed through its normal app lifecycle and the
verified `20260918-204914-333` installation was launched with `--start`.
The UI reached `Running` with the selected PD200X microphone, VB-CABLE output,
and GTCRN configuration; the earlier stale GTCRN error was no longer present.
Clean Voice was then turned off in the live UI, and the saved setting changed to
`passthrough = false`, activating the RVC voice-conversion route for the cable.
The process remained responsive. This establishes current local routing and
activation, but does not prove that another Discord participant can hear the
processed voice or that the model sounds natural.

### Output prebuffer before device playback (2026-09-18)

Realtime startup now begins input capture first, waits up to 500 ms for two
converted output chunk to reach the worker queue, and only then starts the main
output and monitor streams. This avoids consuming an empty output ring while
model loading and first inference are still underway. A timeout remains bounded
and starts playback rather than hanging startup; normal steady-state scheduling
is unchanged.

Release verification with the LJ model, GTCRN, Windows ML, 100 ms chunks, and
output gain zero reached Running for approximately 10 seconds after startup.
It reported zero input overruns and zero dropped output samples; startup output
underruns fell from 20 in the previous comparable run to 2 and stayed at 2.
The run used physical microphone/headphones and did not save or route audio to
Discord. A follow-up release run with the two-chunk target reached Running with
zero input overruns, zero output underruns, and zero dropped samples for its
15-second interval. The release artifact, installed runtime doctor, installed hash, Start
Menu shortcut, and login `--start` shortcut all passed verification. The current
GUI process was preserved; the new build activates on its next launch. Live
voice naturalness, long-run behavior, and Discord reception remain unresolved
acceptance items.

### Input processing allocation reuse on voice resume (2026-09-18)

The RVC stream reset now retains the configured input resampler and its FFT
plans rather than reconstructing the complete stream state. It clears waveform,
pitch, volume/silence history, fixed FIFO and filter state, resets delay trimming,
and resets the existing random/NSF phase and GTCRN state. Independent reference
resampling already resets on each window and keeps its reusable storage.
Sample-rate/chunk changes still follow the existing rebuild path.

Added a reset-versus-fresh resampler regression covering 16/44.1/48/96 kHz,
30 ms hops, prior audio, and subsequent varying samples. Core/app/GUI test targets
compiled with Windows ML/RNNoise/GTCRN; tests were not executed. This source
change is not installed. Live transition quality and timing remain unverified.

### Recording playback route guard (2026-09-18)

Code inspection confirmed the recording picker already runs on a background
thread. Found instead that both recording Play actions used pending device
settings and could target the main output despite the headphone-only UI promise.
They now share one handler using the applied settings while Running and an
AudioFilePlayer monitor-only entry point that rejects empty/whitespace or
main-output headphone selections before enqueuing playback. Normal output
playback remains available to soundboard callers through its existing API.
Added a guard regression case checking rejection does not advance playback
generation or start loading. Live routing verification and installation of this
specific change remain pending; automated tests have not been executed.

The subsequent GUI/CLI release build and installed runtime diagnostic passed.
The recording-route safeguard is now in the side-by-side installation selected
by Start Menu and login shortcuts, with `--start` retained for login. Installed
GUI hash matches the release artifact; the normal settings hash stayed unchanged.
The existing GUI process was preserved, so the safeguard activates on the next
normal launch. Actual recording playback routing still needs a live check.

### Current installation handoff (2026-09-18)

Installed the UI-verified WAV import build side by side, including the microphone
meter correction and output-reset allocation reuse. Installed Windows ML runtime
diagnostics passed with DirectML/CPU fallback; no supported catalog execution
provider was found, and the non-NVIDIA warning is expected on this PC.
The Start Menu and login shortcuts target the new build, with login `--start`
preserved. The original running GUI was not restarted. Therefore these changes
are available on the next normal launch, not active in that existing process.
Earlier section notes about pending installation are superseded by this handoff;
live resume quality, dropout behavior, and remote Discord reception remain
unverified. No production-readiness claim is made.

### Personal WAV soundboard import (2026-09-18)

Added an Add WAV sound action with a background native picker and decoding/write
worker. Shared playback decoding validates the existing duration/sample limits,
then saves a mono float WAV under the user's Sooara soundboard data directory.
The original source is untouched; create-new semantics reject duplicate names.
User clips join bundled clips in the compact grid and use the existing one-shot
and headphone-preview routes. Personal storage survives side-by-side upgrades.
Library refresh waits for an in-progress initial scan rather than losing the
import refresh request. MP3/OGG import is not implemented.

Added tests for duplicate/source preservation, decoded sample preservation,
picker cancellation, error, completion, and disconnect. App/GUI test targets
compile and the diff passes whitespace checks; tests were not executed. This
feature is not installed or UI-verified yet. No real user clip was imported and
no sound was played during implementation.

Subsequent release-build UI verification used the computer-use skill with the
existing isolated APPDATA profile and unavailable diagnostic audio endpoints.
Imported bundled `combo_breaker.wav`: a personal tile appeared, the picker
returned to enabled state, and a 305716-byte normalized WAV was created only in
that profile. A second import showed the file-exists error without changing its
SHA256 (`45AD1582121D52623097B0E94645FF79A0ADAF3F1C7468DA2D4D2F16658553AE`).
The app remained Stopped; no clip was played. Closed the diagnostic app and
verified the normal GUI settings hash was unchanged and the original GUI process
remained responding. The feature is UI-verified for import and duplicate rejection,
but not yet installed in the normal user's shortcut target. Automated tests
remain unexecuted.

### Retained output processing allocations on voice resume (2026-09-18)

The shared chunk converter now resets its smoother and output resampler in
place instead of dropping them during warmup and every return from Clean Voice.
This retains FFT plans and buffer capacity while clearing join history, filter
state, pending samples, counters, and startup delay. Model-rate changes continue
to rebuild both components through the existing rate check. Input/model context
reset remains unchanged and can still contribute switch-time work.

Added a SOLA/PSOLA regression case comparing resumed 40 kHz-to-48 kHz output
against a fresh converter across multiple chunks. Core/app/GUI test targets
compiled with Windows ML, RNNoise, and GTCRN; tests were not executed. Changed
core files pass rustfmt and the diff passes whitespace checks. This optimization
is not installed: real switching latency and audio-quality validation remain
outstanding, and no reduction in audible dropouts is claimed.

The GUI/CLI release build subsequently passed. Converted the preserved dry
recording offline using GTCRN, CPU Windows ML, 100 ms chunks, and the same LJ
model/settings as the earlier SOLA reference. The new WAV is byte-identical to
that reference (SHA256
`B842B37A7A7A0EFEBF0490EA11F14CAD407DB55972E4828049E2ECFFDED839C7`).
This confirms unchanged steady offline conversion for that recording, not live
resume behavior: finite conversion does not exercise the reset operation.
The existing reset regression now expects the known resampler delay to remain
available after reset, while retaining its fresh-output equivalence assertion.
Its updated test target compiled without executing tests. No audio was sent to
Discord and the running GUI was not restarted.

### Microphone meter before noise suppression (2026-09-18)

Normal-screen input RMS now uses a separate worker-side device measurement,
after input gain but before clipping or noise suppression, matching the input
peak warning. Existing post-denoiser pipeline RMS remains unchanged for other
consumers. This prevents successful suppression from hiding microphone activity
on the input bar. No audio samples, gain, or callback work were changed.

The Windows ML/RNNoise/GTCRN vc-app and vc-gui test targets compiled; tests were
not executed. Added coverage for independent raw/processed readings and reset.
GUI/CLI release builds and the side-by-side installed runtime diagnostic passed.
The installer retained login startup with `--start`; the existing running app
was not restarted, so the new meter takes effect on the next normal launch.
Formatting inspection also found pre-existing recovery-block formatting
differences, left untouched. Live microphone-meter rendering remains unverified.

### Headphone-only soundboard preview (2026-09-18)

Sound tiles now expose a right-click Preview in headphones only action. The
shared Soundboard service rejects missing or identical main/monitor endpoints,
stops existing clip playback, and sends the new clip only through its monitor
player. Ordinary left-click one-shot playback is unchanged. Preview uses the
applied route when the engine is running, not an unapplied output selection.
Monitor playback also highlights its matching tile.

GUI/CLI release builds, installed doctor, and compilation of vc-app/vc-gui test
code passed; no automated tests were executed. Using the computer-use skill in
the isolated profile, selected real headphones with a deliberately unavailable
main-output name. Opened the context menu and previewed a bundled clip; the UI
remained responsive and showed no playback error afterward. This checks the
action and error path, not acoustic headphone output or remote-call reception.
The isolated app was then closed. The real running app and Discord routing were
not changed; the preview update is installed for the next normal launch.

### Nonblocking model selection (2026-09-18)

The RVC/ContentVec/RMVPE chooser no longer calls the native file dialog on the
GUI thread. A named worker owns the dialog and returns its result over a channel;
the GUI polls without waiting and applies the existing ONNX/PTH selection flow.
Only one model dialog can be pending. Cancellation leaves the selected model
unchanged; unexpected worker disconnection becomes an actionable UI error.
This follows the existing recording-picker worker pattern, with picker state
kept in a separate module rather than the widget renderer.

Release build, installed doctor, and `cargo check -p vc-gui --tests` with the
Windows ML/RNNoise/GTCRN feature set passed. This compiles but does not execute
tests. In an isolated profile, the native model dialog remained open while the
main GUI repainted and successfully handled Stop. Cancelling re-enabled the
chooser without selecting a file; reopening and choosing the existing ONNX
updated its displayed filename while the engine remained Stopped. The
computer-use skill verified these interactions. No audio was started in that
profile, and the user's active session was not restarted. The new build is
installed for next launch. PTH conversion after asynchronous selection and
all platform-specific native dialog behaviors are not yet runtime-verified.

### Matched SOLA / PSOLA recorded-voice comparison (2026-09-18)

Converted the preserved 20.64-second dry take through the shared WAV pipeline,
using the local LJ RVC v2 model, ContentVec, RMVPE, GTCRN, Windows ML CPU,
100 ms chunks, 100 ms extra context, 85 ms requested crossfade, 12 ms search,
10 ms tail discard, unity gains, and zero pitch shift. Only smoothing mode
changed. This is an offline CPU comparison, not the live GPU's latency result.

Both outputs contain 990720 mono PCM16 samples at 48 kHz and no clipped samples.
SOLA peak/RMS were -9.992/-34.749 dBFS; PSOLA -9.988/-34.767 dBFS. Across 206
reported joins, SOLA median/p95/max boundary-step ratios were 0.930/3.497/7.299;
PSOLA 0.942/3.748/7.790. Mean overlap correlation was 0.5341 versus 0.5223.
Mean absolute short-window energy change was 3.074 versus 2.895 dB; PSOLA fell
back on 68 joins. Ratios compare boundary steps to nearby waveform differences,
not human quality scores. Mixed results do not justify changing the live SOLA
setting, and neither smoothness metric establishes naturalness/intelligibility.

Comparison WAVs and CSVs are ignored local assets named
`assets/sooara-gtcrn-{sola,psola}-20260918.*`; originals and live settings were
not changed. Conversion commands completed successfully; automated tests were
not run. Fresh model information in the running GUI also read 105.1 MiB,
40000 Hz, v2 successfully, so the earlier cached file-not-found display did not
reproduce. No metadata-reader change was made on that evidence.

### Discord routing corrected while muted (2026-09-17)

Current UI inspection found Discord connected with the user's microphone muted.
Its selected input was still the physical microphone, bypassing Sooara. Through
the computer-use skill, selected the capture side of the existing virtual cable
and changed the Input Profile from Voice Isolation to Studio (shown by Discord
as no processing), avoiding a second isolation stage after Sooara's GTCRN.
Headphone output was unchanged. Returned to the call and verified the mute
indicator remained enabled. No Mic Test, soundboard injection, unmute, message,
or call join/leave action was performed.

Inspection of Sooara then found Stopped, not Running: a responsive process was
not sufficient evidence of an active audio session. Closed that stopped older
build and launched the latest installed GUI with `--start`. It reached Running
in Clean Voice with the saved physical microphone and virtual-cable output.
This launch also activates the recent recovery/startup-wait changes.

A short FFmpeg capture of the virtual-cable capture endpoint was sent only to
the null sink, with no audio file retained. It received stereo 44.1 kHz PCM,
peak -63.46 dBFS and overall RMS -87.24 dBFS in this quiet observation. Nonzero
low-level samples establish an active endpoint, not intelligible speech,
speaker identity, denoising quality, or remote Discord reception. Gain was not
changed on that basis. Discord remains muted for the user to control; remote
listening and natural converted voice quality are still outstanding.

### Wait for unavailable devices at startup (2026-09-17)

Recovery-enabled startup now checks the selected endpoints before opening the
session. Missing/ambiguous devices or an enumeration failure enter the same
cancellable three-second polling flow as a disconnected session. Models are not
loaded repeatedly while devices are absent. Explicit Stop cancels the pending
start; a later Start is a new request. Normal CLI behavior remains unchanged.
If endpoints are initially available, their resolved names are checked again
before streams start. Non-device model/runtime errors still surface as errors.

GUI/CLI release builds and installed doctor passed. In an isolated APPDATA profile
with deliberately nonexistent input/output names, the installed GUI displayed
Starting / Waiting for the selected audio devices. Stop changed it to Stopped,
and a subsequent observation remained Stopped. The real profile's SHA-256 was
unchanged, and its running process was not restarted. No audio was captured or
sent by the isolated profile. The computer-use skill was used to verify the
waiting and cancellation UI. Added polling/enumeration regression tests were
not executed. Actual device appearance and subsequent live audio remain to be
verified; this check does not establish successful physical reconnection.

### Device recovery and explicit-route safety (2026-09-17)

The GUI opts into shared-runtime device recovery. A stream failure from a
previously running session preserves its configuration and latest passthrough
choice, releases the failed session, and polls selected endpoints every three
seconds on the control thread. It waits for unambiguous exact names, including
the monitor when configured, then reuses the normal session startup path.
Recovered input/output names are checked again before streams start. Stop,
replacement configuration, device diagnostics, or disabling recovery cancels
the pending recovery. Failed reopen attempts are capped at three. Missing
endpoints remain in a cancellable waiting state without repeatedly loading models.
CLI recovery remains off unless explicitly enabled through the controller API.

Explicit CPAL device selections no longer fall back to the system default when
not found. Exact names take priority over partial-name CLI matching. Output
diagnostics/monitoring additionally verify the resolved endpoint before starting
its stream, closing a disconnect-between-enumeration-and-open routing race.
Sessions using default devices or debug WAV capture do not auto-recover.
Initial startup failure and inference-worker failure are not automatically retried.

GUI/CLI release builds passed. Running the CLI with a nonexistent input and then
a nonexistent output returned explicit device-not-found errors (exit 1), rather
than opening the default route. The side-by-side installer passed runtime doctor
and retained the login `--start` shortcut. The already-running process remains
on the preceding build until next launch. Recovery-policy tests were added but
not run. Physical unplug/replug, recovery cancellation in the GUI, and recovered
audio quality remain unverified; no hardware was disconnected for this check.

### Protect active audio from accidental exit (2026-09-17)

Closing the GUI while the engine is Starting, Running, or Stopping now opens a
modal with Keep running / Minimize, Stop and exit, and Cancel. Dismissing the
modal keeps the session alive. The lifecycle guard owns confirmation state;
the dialog only dispatches choices. Stopped/error sessions retain normal exit,
and a failed settings save clears exit confirmation before cancelling close.
This is taskbar minimization, not a system-tray implementation or crash recovery.

The Windows ML release build and installed runtime diagnostic passed. Using the
computer-use skill, an actual running session displayed the dialog on Alt+F4;
Keep running minimized it, and restoring showed Running with the same process.
Stop and exit subsequently terminated that process. The updated login shortcut
retains `--start`. Added lifecycle unit tests were not executed.

Discord was inspected read-only during an active call/screen share. Its selected
microphone was Windows Default (the physical microphone), bypassing Sooara's
virtual-cable output, and its Input Profile was Voice Isolation. No call routing
or processing settings were changed. Downstream Discord reception therefore
remains unverified; the app's Running state does not establish that Discord is
receiving its output.

### Reuse GTCRN on return to Clean Voice (2026-09-17)

The passthrough reset path was reconstructing the GTCRN ONNX session whenever
switching back from RVC. It now resets the loaded denoiser's caches/adapter and
reuses its session, matching the existing RVC reset behavior. Initial load still
validates and loads the model normally. Resampling history is reset in both cases.
This removes model disk/session work from a live mode change, but does not remove
the different route latencies or prove a seamless transition.

GUI/CLI release builds and installed doctor passed. Added an explicitly ignored
CPU-ORT regression test that initializes GTCRN, clears the processor's model-path
setting, then resets and processes again. The test was not run. Updated files are
installed for next launch; the already-running process still needs a relaunch
and a before/after transition check.

### Live GTCRN selection (2026-09-17)

Selected GTCRN through the installed GUI, applied it with Restart, and verified
the persisted local setting. The app reached Running with the existing physical
microphone, virtual-cable output, and separate monitor headphones. Monitoring
was enabled through Hear myself. In Clean Voice, displayed processing was about
7–8 ms per 100 ms chunk; input overruns and monitor drops were zero in the observed
snapshot. Monitor callback-consumed samples increased.

Turning off Clean Voice activated neural conversion without restarting the GUI.
Processing then displayed 71.3 ms per 100 ms chunk and nominal content delay
187.81 ms, excluding device/queue latency. Main output underruns rose from 14
to 20 and monitor missing samples from 2880 to 5760 around the switch. This
does not pass seamless-switch acceptance. No input overruns or monitor drops
appeared in that snapshot. Restored Clean Voice, retaining GTCRN and monitoring
for this session. Monitoring remains deliberately muted after a future launch.
This verifies live paths and saved selection, not audible quality or Discord.

### Reopening the running app (2026-09-17)

The instance-lock owner now writes a per-user PID sidecar. A duplicate launch
enumerates only that process's Sooara window and asks Windows to show/restore
and foreground it, then exits without creating another engine. A retained PID
file alone never establishes that an instance is running: activation is attempted
only after the OS file lock reports contention. Windows may restrict foreground
focus; this does not bypass OS focus rules.

The updated release built and was installed. Its startup command reached Running.
After minimizing the window, a second installed process exited with code 0 and
the original window was restored without a separate automation activation call;
the original process remained responsive and displayed Running. The running build
also now includes the earlier device-failure visibility changes. Lock regression
tests remain unexecuted. Tray behavior, automatic device recovery, perceptual
voice quality, and Discord reception are still not completed.

### Recorded-microphone denoiser comparison (2026-09-17)

Added `vc-app`'s `denoise_recording` example for local mono PCM16 recordings.
It uses the existing shared finite RNNoise adapter and exposes the same finite
adapter path for GTCRN, preserving frame count and removing reported streaming
delay. It refuses existing output files and does not alter input recordings.
Build with `--no-default-features --features windowsml,rnnoise,gtcrn`; development
examples need `VC_RS_WINDOWSML_BOOTSTRAP_DLL` pointing to the built bootstrap DLL.

Both algorithms processed the preserved 20.64-second dry take into 990720 samples
at 48 kHz. Using 100 ms windows selected by the original input's energy, the
quietest 20 percent had mean energy reduced by 9.41 dB with RNNoise and 11.47 dB
with GTCRN. The loudest 20 percent changed by -0.097 and -0.092 dB respectively.
These are energy-selected windows, not manually labelled speech/noise, so they
do not establish intelligibility or noise-only attenuation. Local processing
including initialization took 79 ms (RNNoise) and 1375 ms (GTCRN); these are
offline throughput figures, not microphone-to-headphone latency.

A second diagnostic added FFmpeg pink noise (48 kHz, amplitude 0.02, seed 7341,
20.64 s) without normalization. Relative to the original take after a common
7 kHz low-pass filter, reference SNR was 14.68 dB for the mixture, 13.01 dB after
RNNoise, and 16.60 dB after GTCRN. No clipped samples were found. The original
take itself contains noise, and this single synthetic-noise condition is not a
clean-reference benchmark, perceptual evaluation, or Krisp comparison. GTCRN's
16 kHz internal rate also limits bandwidth. Results favor trying GTCRN for this
recording but do not establish a universal default. Comparison WAVs remain local
and ignored under `assets/sooara-denoise-*-20260917.wav`. Tests were not run.

### Device failure visibility (2026-09-17)

Inspection found that live sessions only logged stream errors, leaving the GUI
able to report Running after a device failed. File playback could likewise stay
Playing when its callback stopped. Both paths now inspect stream failure and
stop the affected session with an actionable reconnect/restart message. The
control thread identifies microphone, main-output, or monitor failure; callbacks
still only update atomics. CPAL non-xrun failure state is latched until the stream
is replaced so diagnostic counter draining cannot erase a failure. Ordinary
xruns remain nonfatal, matching the existing device-test distinction.

GUI and CLI release builds passed. The failure-latch regression test was added
but not executed. Physical disconnection/reconnection has not been verified,
and automatic reconnection remains unfinished. The update was installed side by
side; the previously running instance is not hot-patched by installation.

### Automatic saved-session startup (2026-09-17)

The GUI now accepts `--start`. It uses the existing engine start path with the
saved configuration, only when settings loaded without error, setup/terms do not
need attention, and explicit input/output devices are selected. Otherwise the
window remains available with an explanation. Monitoring still starts muted.
Missing devices or models surface through normal engine validation; automatic
retry/device recovery is not implemented yet.

A per-user OS-held file lock prevents a second new GUI instance from starting
another engine. The lock releases on process exit; the file may remain without
blocking the next launch. It does not coordinate with older builds or the CLI.
Duplicate launches currently exit without bringing the existing window forward.

The local installer accepts `-StartAudioAtLogin` to set `--start` on its per-user
Startup shortcut. Explicit `-StartAudioAtLogin:$false` restores launch-only mode;
omitting it preserves the existing preference across updates. The setting is
enabled on this PC. A launch using the installed shortcut's target and arguments
reached Running in the actual GUI without a Start click. A second `--start`
process exited with code 0, leaving the original process responsive. This verifies
the launch command, not a full Windows login/reboot cycle or acoustic quality.

The release build and installed doctor passed. A lock exclusion/release unit
test was added but not executed, per the user's testing instruction. Background
tray operation, foreground activation, crash recovery, and device reconnect
remain unfinished.

### Installed GUI and login availability (2026-09-17)

The native computer-use runtime became available. First-run setup was completed
in the installed window using enumerated PD200X input, the existing virtual cable
as main output, and Realtek headphones as independent monitor. The cached local
candidate voice and support models were selected; settings persisted across
closing and reopening the app. No personal device names were added to source.

The GUI reached Running. Clean Voice and Hear myself toggles responded without
restarting the engine. Neural conversion at the saved 100 ms chunk setting showed
approximately 60–62 ms processing and zero input overruns. Output underruns rose
from 20 to 21 across observations, so this is not a dropout-free acceptance pass.
Nominal content delay was 169.81 ms, not measured end-to-end latency. Naturalness,
noise suppression quality, and Discord remote reception remain unverified.

An existing recording played inside the app and Stop returned playback to idle
while the engine remained Running, without opening an external media player.
Visual inspection found long sound names overflowing the grid. Tiles now have
bounded equal widths and wrapped labels; the rebuilt installed window displayed
eight columns at its normal width and ten when maximized. Recording row heights
now account for button text and padding. A soundboard click was exercised without
an error; acoustic playback was not assessed in this GUI check.

The installer now accepts `-LaunchAtLogin` and preserves an existing login launch
on subsequent installs. Both Start menu and per-user Startup shortcuts were
verified to point to the installed, hash-checked preview. It opens the app only;
audio processing still requires Start. Actual login/reboot behavior has not been
tested, and continuous background operation/recovery remains unfinished.
The release build and installed runtime diagnostic passed; tests were not run.

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
