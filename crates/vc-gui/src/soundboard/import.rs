use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};

#[derive(Default)]
pub(super) struct SoundImport {
    pending: Option<Receiver<Result<Option<PathBuf>, String>>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_pending_cancel_and_error_release_picker() {
        let (tx, rx) = mpsc::channel();
        let mut importer = SoundImport { pending: Some(rx) };
        assert!(importer.poll().unwrap().is_none());
        assert!(importer.active());
        tx.send(Ok(None)).unwrap();
        assert!(importer.poll().unwrap().is_none());
        assert!(!importer.active());
        let (tx, rx) = mpsc::channel();
        importer.pending = Some(rx);
        tx.send(Err("invalid WAV".into())).unwrap();
        assert_eq!(importer.poll().unwrap_err(), "invalid WAV");
        assert!(!importer.active());
    }

    #[test]
    fn import_completion_and_disconnect_release_picker() {
        let (tx, rx) = mpsc::channel();
        let mut importer = SoundImport { pending: Some(rx) };
        tx.send(Ok(Some(PathBuf::from("sound.wav")))).unwrap();
        assert_eq!(importer.poll().unwrap(), Some(PathBuf::from("sound.wav")));
        assert!(!importer.active());
        let (tx, rx) = mpsc::channel();
        importer.pending = Some(rx);
        drop(tx);
        assert!(importer.poll().is_err());
        assert!(!importer.active());
    }
}

impl SoundImport {
    pub(super) fn active(&self) -> bool {
        self.pending.is_some()
    }

    pub(super) fn start(&mut self) -> Result<(), String> {
        if self.active() {
            return Ok(());
        }
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("sooara-sound-import".into())
            .spawn(move || {
                let result = rfd::FileDialog::new()
                    .add_filter("WAV sound", &["wav"])
                    .pick_file()
                    .map(|path| vc_app::import_soundboard_clip(&path))
                    .transpose()
                    .map_err(|error| format!("{error:#}"));
                let _ = tx.send(result);
            })
            .map_err(|error| format!("Could not open sound import: {error}"))?;
        self.pending = Some(rx);
        Ok(())
    }

    pub(super) fn poll(&mut self) -> Result<Option<PathBuf>, String> {
        let Some(pending) = self.pending.as_ref() else {
            return Ok(None);
        };
        match pending.try_recv() {
            Ok(result) => {
                self.pending = None;
                result
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                self.pending = None;
                Err("Sound import closed unexpectedly. Try again.".into())
            }
        }
    }
}
