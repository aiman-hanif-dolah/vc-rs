use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct RecordingEntry {
    pub path: PathBuf,
    pub name: String,
    pub duration_seconds: Option<f64>,
    pub bytes: u64,
    modified: Option<SystemTime>,
}

#[derive(Clone, Debug, Default)]
pub struct RecordingLibrarySnapshot {
    pub entries: Vec<RecordingEntry>,
    pub loading: bool,
    pub error: Option<String>,
}

pub struct RecordingLibrary {
    directory: PathBuf,
    state: Arc<Mutex<RecordingLibrarySnapshot>>,
}

pub fn recordings_directory() -> Result<PathBuf> {
    std::env::var_os("APPDATA")
        .map(|root| PathBuf::from(root).join("Sooara").join("recordings"))
        .context("APPDATA is unavailable; cannot locate recordings")
}

impl RecordingLibrary {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            state: Arc::new(Mutex::new(RecordingLibrarySnapshot::default())),
        }
    }

    pub fn refresh(&self) -> Result<()> {
        {
            let mut state = self.state.lock().unwrap();
            if state.loading {
                return Ok(());
            }
            state.loading = true;
            state.error = None;
        }
        let directory = self.directory.clone();
        let state = Arc::clone(&self.state);
        let result = std::thread::Builder::new()
            .name("sooara-recording-library".into())
            .spawn(move || {
                let result = scan(&directory);
                let mut state = state.lock().unwrap();
                state.loading = false;
                match result {
                    Ok(entries) => state.entries = entries,
                    Err(error) => state.error = Some(format!("{error:#}")),
                }
            });
        if let Err(error) = result {
            self.state.lock().unwrap().loading = false;
            return Err(error.into());
        }
        Ok(())
    }

    pub fn snapshot(&self) -> RecordingLibrarySnapshot {
        self.state
            .lock()
            .map(|state| state.clone())
            .unwrap_or_default()
    }
}

fn is_recording(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
}

fn scan(directory: &Path) -> Result<Vec<RecordingEntry>> {
    let listing = match std::fs::read_dir(directory) {
        Ok(listing) => listing,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut entries = Vec::new();
    for entry in listing {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_file() || !is_recording(&path) {
            continue;
        }
        let metadata = entry.metadata()?;
        let duration_seconds = hound::WavReader::open(&path).ok().and_then(|reader| {
            let rate = reader.spec().sample_rate;
            (rate > 0).then(|| f64::from(reader.duration()) / f64::from(rate))
        });
        entries.push(RecordingEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            path,
            duration_seconds,
            bytes: metadata.len(),
            modified: metadata.modified().ok(),
        });
    }
    entries.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_extension_is_case_insensitive_and_exact() {
        assert!(is_recording(Path::new("take.WAV")));
        assert!(is_recording(Path::new("take.wav")));
        assert!(!is_recording(Path::new("take.wav.tmp")));
        assert!(!is_recording(Path::new("take.mp3")));
    }
}
