use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use vc_app::{AudioFilePlayer, AudioHost};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let first = args
        .next()
        .context("usage: play_recording <wav> <output-device> | --list")?;
    if first == "--list" {
        return list_recordings();
    }
    let path = PathBuf::from(first);
    let device = args.next().context("output device is required")?;
    let player = AudioFilePlayer::default();
    player.play(path.clone(), AudioHost::default(), device)?;
    let started = Instant::now();
    loop {
        let state = player.snapshot();
        if let Some(error) = state.error {
            bail!("{error}");
        }
        if state.path.as_ref() == Some(&path) && !state.loading && !state.playing {
            println!("Playback completed without looping");
            return Ok(());
        }
        if started.elapsed() > Duration::from_secs(660) {
            bail!("Playback did not finish");
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn list_recordings() -> Result<()> {
    let library = vc_app::RecordingLibrary::new(vc_app::recordings_directory()?);
    library.refresh()?;
    let started = Instant::now();
    loop {
        let state = library.snapshot();
        if let Some(error) = state.error {
            bail!("{error}");
        }
        if !state.loading {
            println!("{} recordings", state.entries.len());
            for entry in state.entries {
                println!(
                    "{}: {:?} seconds, {} bytes",
                    entry.name, entry.duration_seconds, entry.bytes
                );
            }
            return Ok(());
        }
        if started.elapsed() > Duration::from_secs(30) {
            bail!("Recording scan timed out");
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}
