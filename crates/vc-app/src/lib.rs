//! Shared application runtime used by the CLI and standalone GUI.
//!
//! Frontends communicate with [`EngineController`]. Audio callbacks only touch
//! preallocated lock-free sample rings and atomics; they must never be coupled
//! to GUI rendering, model loading, or other blocking work.

pub mod audio;
mod playback;
mod realtime;
mod recording_library;
mod soundboard;

pub use soundboard::{soundboard_directory, Soundboard};

pub use playback::{AudioFilePlayer, PlaybackSnapshot};
pub use recording_library::{
    recordings_directory, RecordingEntry, RecordingLibrary, RecordingLibrarySnapshot,
};

pub use realtime::{
    write_wav_mono, AudioHost, DenoiserMode, DeviceList, DeviceTestConfig, DeviceTestSnapshot,
    EngineController, EngineState, EngineStatusSnapshot, RealtimeConfig, Smoother,
    TelemetrySnapshot, TestOutput,
};
pub use vc_core::model_rvc::{F0Config, LiveParams, NoiseGateShaping, OutputDynamicsConfig};
