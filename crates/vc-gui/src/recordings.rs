use super::*;
use std::sync::mpsc::{self, Receiver};

#[derive(Default)]
pub(crate) struct RecordingControls {
    player: vc_app::AudioFilePlayer,
    picker: Option<Receiver<Option<PathBuf>>>,
    selected: Option<PathBuf>,
    library: Option<vc_app::RecordingLibrary>,
    library_initialized: bool,
}

impl VcGui {
    pub(crate) fn recordings_ui(&mut self, ui: &mut egui::Ui) {
        if !self.recordings.library_initialized {
            self.recordings.library_initialized = true;
            match vc_app::recordings_directory() {
                Ok(directory) => {
                    let library = vc_app::RecordingLibrary::new(directory);
                    if let Err(error) = library.refresh() {
                        self.ui_error = Some(error.to_string());
                    }
                    self.recordings.library = Some(library);
                }
                Err(error) => self.ui_error = Some(error.to_string()),
            }
        }
        if let Some(picker) = &self.recordings.picker {
            match picker.try_recv() {
                Ok(path) => {
                    if path.is_some() {
                        self.recordings.selected = path;
                    }
                    self.recordings.picker = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => self.recordings.picker = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        let snapshot = self.recordings.player.snapshot();
        ui.heading("Recordings");
        if let Some(library) = &self.recordings.library {
            let state = library.snapshot();
            if ui
                .add_enabled(!state.loading, egui::Button::new("Refresh recordings"))
                .clicked()
            {
                if let Err(error) = library.refresh() {
                    self.ui_error = Some(error.to_string());
                }
            }
            if state.loading {
                ui.label("Scanning recordings…");
            }
            if let Some(error) = &state.error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
            if state.entries.is_empty() && !state.loading {
                ui.label("No recordings found. You can choose a WAV file below.");
            }
            let mut play = None;
            egui::ScrollArea::vertical()
                .id_salt("recording-library")
                .max_height(200.0)
                .show_rows(ui, 26.0, state.entries.len(), |ui, rows| {
                    for index in rows {
                        let entry = &state.entries[index];
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(
                                    !self.settings.monitor_device.is_empty(),
                                    egui::Button::new("Play"),
                                )
                                .clicked()
                            {
                                play = Some(entry.path.clone());
                            }
                            ui.label(&entry.name);
                            ui.weak(
                                entry
                                    .duration_seconds
                                    .map(|seconds| format!("{seconds:.1}s"))
                                    .unwrap_or_else(|| "Unreadable WAV".into()),
                            );
                        });
                    }
                });
            if let Some(path) = play {
                self.recordings.selected = Some(path.clone());
                if let Err(error) = self.recordings.player.play(
                    path,
                    self.settings.output_host(),
                    self.settings.monitor_device.clone(),
                ) {
                    self.ui_error = Some(error.to_string());
                }
            }
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.recordings.picker.is_none(),
                    egui::Button::new("Choose recording…"),
                )
                .clicked()
            {
                let (tx, rx) = mpsc::channel();
                self.recordings.picker = Some(rx);
                std::thread::spawn(move || {
                    let mut dialog = rfd::FileDialog::new().add_filter("WAV recordings", &["wav"]);
                    if let Ok(directory) = vc_app::recordings_directory() {
                        dialog = dialog.set_directory(directory);
                    }
                    let _ = tx.send(dialog.pick_file());
                });
            }
            let ready =
                self.recordings.selected.is_some() && !self.settings.monitor_device.is_empty();
            if ui
                .add_enabled(ready && !snapshot.loading, egui::Button::new("Play once"))
                .clicked()
            {
                if let Some(path) = self.recordings.selected.clone() {
                    if let Err(error) = self.recordings.player.play(
                        path,
                        self.settings.output_host(),
                        self.settings.monitor_device.clone(),
                    ) {
                        self.ui_error = Some(error.to_string());
                    }
                }
            }
            if ui
                .add_enabled(
                    snapshot.playing || snapshot.loading,
                    egui::Button::new("Stop playback"),
                )
                .clicked()
            {
                if let Err(error) = self.recordings.player.stop() {
                    self.ui_error = Some(error.to_string());
                }
            }
        });
        if let Some(path) = &self.recordings.selected {
            ui.label(path.file_name().unwrap_or_default().to_string_lossy());
        }
        if snapshot.loading {
            ui.label("Loading recording…");
        }
        if snapshot.playing {
            ui.label("Playing recording through monitor headphones");
        }
        if let Some(error) = snapshot.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        ui.small("Uses the headphones selected under Audio. Playback stays local and is not sent to your main voice output.");
    }
}
