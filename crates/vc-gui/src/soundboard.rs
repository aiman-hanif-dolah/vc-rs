use super::*;
use vc_app::EngineStatusSnapshot;

#[derive(Default)]
pub(crate) struct SoundboardControls {
    player: vc_app::Soundboard,
    library: Option<vc_app::RecordingLibrary>,
    initialized: bool,
    error: Option<String>,
}

impl VcGui {
    pub(crate) fn soundboard_ui(&mut self, ui: &mut egui::Ui, status: &EngineStatusSnapshot) {
        if !self.soundboard.initialized {
            self.soundboard.initialized = true;
            match vc_app::soundboard_directory() {
                Ok(directory) => {
                    let library = vc_app::RecordingLibrary::new(directory);
                    if let Err(error) = library.refresh() {
                        self.soundboard.error = Some(error.to_string());
                    }
                    self.soundboard.library = Some(library);
                }
                Err(error) => self.soundboard.error = Some(error.to_string()),
            }
        }
        ui.heading("Soundboard");
        let state = self.soundboard.player.snapshot();
        if ui.button("Stop sounds").clicked() {
            if let Err(error) = self.soundboard.player.stop() {
                self.soundboard.error = Some(error.to_string());
            }
        }
        let mut selected = None;
        if let Some(library) = &self.soundboard.library {
            let library = library.snapshot();
            if library.loading {
                ui.label("Loading sounds…");
            }
            if let Some(error) = &library.error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
            let columns = ((ui.available_width() / 100.0).floor() as usize).clamp(1, 10);
            let ready =
                status.state == EngineState::Running || !self.settings.output_device.is_empty();
            egui::Grid::new("soundboard-grid")
                .num_columns(columns)
                .spacing([6.0, 6.0])
                .show(ui, |ui| {
                    for (index, entry) in library.entries.iter().enumerate() {
                        let playing = state.playing && state.path.as_ref() == Some(&entry.path);
                        let label = entry
                            .path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .replace('_', " ");
                        if ui
                            .add_enabled(
                                ready,
                                egui::Button::new(label)
                                    .selected(playing)
                                    .min_size(egui::vec2(88.0, 40.0)),
                            )
                            .clicked()
                        {
                            selected = Some(entry.path.clone());
                        }
                        if (index + 1) % columns == 0 {
                            ui.end_row();
                        }
                    }
                });
        }
        if let Some(path) = selected {
            self.play_soundboard(path, status);
        }
        let monitor = self.soundboard.player.monitor_snapshot();
        for error in [
            self.soundboard.error.as_ref(),
            state.error.as_ref(),
            monitor.error.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        ui.small("Click once to play once. Sounds go to your main output and selected monitor headphones. A new sound replaces the previous one.");
    }

    fn play_soundboard(&mut self, path: PathBuf, status: &EngineStatusSnapshot) {
        let settings = if status.state == EngineState::Running {
            self.onboarding
                .normal
                .applied
                .as_ref()
                .unwrap_or(&self.settings)
        } else {
            &self.settings
        };
        let output = if status.state == EngineState::Running {
            status.output_device.clone()
        } else {
            settings.output_device.clone()
        };
        self.soundboard.error = self
            .soundboard
            .player
            .play(
                path,
                settings.output_host(),
                output,
                string_option(&settings.monitor_device),
            )
            .err()
            .map(|error| error.to_string());
    }
}
