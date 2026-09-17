use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};

use super::{ui_text::Language, ModelKind};

#[derive(Default)]
pub(crate) struct ModelPicker {
    pending: Option<Receiver<(ModelKind, Option<PathBuf>)>>,
}

impl ModelPicker {
    pub(crate) fn active(&self) -> bool {
        self.pending.is_some()
    }

    pub(crate) fn start(&mut self, kind: ModelKind, language: Language) -> Result<(), String> {
        if self.active() {
            return Ok(());
        }
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("vc-model-picker".into())
            .spawn(move || {
                let dialog = match kind {
                    ModelKind::Rvc => rfd::FileDialog::new()
                        .add_filter(language.text("RVC model"), &["onnx", "pth"])
                        .add_filter(language.text("ONNX model"), &["onnx"])
                        .add_filter(language.text("PyTorch checkpoint"), &["pth"]),
                    ModelKind::Embedder | ModelKind::F0 => rfd::FileDialog::new()
                        .add_filter(language.text("ONNX model"), &["onnx"]),
                };
                let _ = tx.send((kind, dialog.pick_file()));
            })
            .map_err(|error| format!("Could not open the model picker: {error}"))?;
        self.pending = Some(rx);
        Ok(())
    }

    pub(crate) fn poll(&mut self) -> Result<Option<(ModelKind, PathBuf)>, String> {
        let Some(pending) = self.pending.as_ref() else {
            return Ok(None);
        };
        match pending.try_recv() {
            Ok((kind, path)) => {
                self.pending = None;
                Ok(path.map(|path| (kind, path)))
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                self.pending = None;
                Err("Model picker closed unexpectedly. Choose the model again.".into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_cancel_and_selection_are_nonblocking() {
        let (tx, rx) = mpsc::channel();
        let mut picker = ModelPicker { pending: Some(rx) };
        assert!(picker.poll().unwrap().is_none());
        assert!(picker.active());
        assert!(tx.send((ModelKind::Rvc, None)).is_ok());
        assert!(picker.poll().unwrap().is_none());
        assert!(!picker.active());

        let (tx, rx) = mpsc::channel();
        picker.pending = Some(rx);
        assert!(tx.send((ModelKind::F0, Some(PathBuf::from("pitch.onnx")))).is_ok());
        let (kind, path) = picker.poll().unwrap().unwrap();
        assert!(matches!(kind, ModelKind::F0));
        assert_eq!(path, PathBuf::from("pitch.onnx"));
        assert!(!picker.active());
    }

    #[test]
    fn disconnected_picker_releases_pending_state() {
        let (tx, rx) = mpsc::channel();
        let mut picker = ModelPicker { pending: Some(rx) };
        drop(tx);
        assert!(picker.poll().is_err());
        assert!(!picker.active());
    }
}
