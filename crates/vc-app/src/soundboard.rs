use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::{AudioFilePlayer, AudioHost, PlaybackSnapshot};

#[derive(Default)]
pub struct Soundboard {
    output: AudioFilePlayer,
    monitor: AudioFilePlayer,
}

impl Soundboard {
    pub fn preview(
        &self,
        path: PathBuf,
        host: AudioHost,
        output: String,
        headphones: String,
    ) -> Result<()> {
        let headphones = monitor_target(&output, Some(headphones))
            .context("Choose separate monitor headphones before previewing a sound")?;
        self.stop()?;
        self.monitor.play(path, host, headphones)
    }

    pub fn play(
        &self,
        path: PathBuf,
        host: AudioHost,
        output: String,
        monitor: Option<String>,
    ) -> Result<()> {
        self.monitor.stop()?;
        self.output.play(path.clone(), host, output.clone())?;
        if let Some(monitor) = monitor_target(&output, monitor) {
            self.monitor.play(path, host, monitor)?;
        }
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        let output = self.output.stop();
        let monitor = self.monitor.stop();
        output.and(monitor)
    }

    pub fn snapshot(&self) -> PlaybackSnapshot {
        self.output.snapshot()
    }

    pub fn monitor_snapshot(&self) -> PlaybackSnapshot {
        self.monitor.snapshot()
    }
}

fn monitor_target(output: &str, monitor: Option<String>) -> Option<String> {
    monitor.filter(|device| !device.is_empty() && device != output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_does_not_duplicate_main_output() {
        assert_eq!(monitor_target("Cable", Some("Cable".into())), None);
        assert_eq!(monitor_target("Cable", Some(String::new())), None);
        assert_eq!(
            monitor_target("Cable", Some("Headphones".into())),
            Some("Headphones".into())
        );
    }

    #[test]
    fn preview_rejects_missing_or_main_output_monitor() {
        let soundboard = Soundboard::default();
        for headphones in [String::new(), "Cable".into()] {
            assert!(soundboard
                .preview(
                    PathBuf::from("unused.wav"),
                    AudioHost::default(),
                    "Cable".into(),
                    headphones,
                )
                .is_err());
        }
        assert!(!soundboard.snapshot().playing);
        assert!(!soundboard.monitor_snapshot().playing);
    }
}

pub fn soundboard_directory() -> Result<PathBuf> {
    let executable = std::env::current_exe()?;
    let installed = executable
        .parent()
        .context("Cannot locate application directory")?
        .join("soundboard");
    if installed.is_dir() {
        return Ok(installed);
    }
    let development = std::env::current_dir()?
        .join("resources")
        .join("soundboard");
    if development.is_dir() {
        return Ok(development);
    }
    anyhow::bail!("Soundboard files are missing. Reinstall Sooara with its soundboard folder.")
}
