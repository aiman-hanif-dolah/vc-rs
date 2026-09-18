use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::{AudioFilePlayer, AudioHost, PlaybackSnapshot};

pub fn user_soundboard_directory() -> Result<PathBuf> {
    std::env::var_os("APPDATA")
        .map(|root| PathBuf::from(root).join("Sooara").join("soundboard"))
        .context("APPDATA is unavailable; cannot locate personal sounds")
}

/// Decode and persist on a background worker, never on the audio or UI thread.
pub fn import_soundboard_clip(source: &Path) -> Result<PathBuf> {
    import_clip_into(source, &user_soundboard_directory()?)
}

fn import_clip_into(source: &Path, directory: &Path) -> Result<PathBuf> {
    let (samples, sample_rate) = crate::playback::read_wav(source)?;
    let mut name = source
        .file_stem()
        .context("Sound has no file name")?
        .to_os_string();
    name.push(".wav");
    std::fs::create_dir_all(directory)?;
    let target = directory.join(name);
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
        .context("Cannot add sound; a sound with this name may already exist")?;
    let result = (|| -> Result<()> {
        let mut writer = hound::WavWriter::new(
            file,
            hound::WavSpec {
                channels: 1,
                sample_rate,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )?;
        for sample in samples {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
        Ok(())
    })();
    if let Err(error) = result {
        // Only this call's newly created file is eligible for cleanup.
        let _ = std::fs::remove_file(&target);
        return Err(error);
    }
    Ok(target)
}

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
    fn import_preserves_source_and_rejects_duplicate_name() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("sooara-import-{}-{stamp}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let source = root.join("my.sound.wav");
        let mut writer = hound::WavWriter::create(
            &source,
            hound::WavSpec {
                channels: 1,
                sample_rate: 48_000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        writer.write_sample(8192_i16).unwrap();
        writer.finalize().unwrap();
        let original = std::fs::read(&source).unwrap();
        let directory = root.join("personal");
        let target = import_clip_into(&source, &directory).unwrap();
        assert_eq!(target.file_name().unwrap(), "my.sound.wav");
        let imported = std::fs::read(&target).unwrap();
        assert!(import_clip_into(&source, &directory).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), imported);
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let (samples, rate) = crate::playback::read_wav(&target).unwrap();
        assert_eq!(samples, vec![0.25]);
        assert_eq!(rate, 48_000);
        std::fs::remove_file(target).unwrap();
        std::fs::remove_file(source).unwrap();
        std::fs::remove_dir(directory).unwrap();
        std::fs::remove_dir(root).unwrap();
    }

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
