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
        let monitor = self.soundboard.player.monitor_snapshot();
        if ui.button("Stop sounds").clicked() {
            if let Err(error) = self.soundboard.player.stop() {
                self.soundboard.error = Some(error.to_string());
            }
        }
        let mut selected = None;
        let mut preview = None;
        if let Some(library) = &self.soundboard.library {
            let library = library.snapshot();
            if library.loading {
                ui.label("Loading sounds…");
            }
            if let Some(error) = &library.error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
            let columns = ((ui.available_width() / 100.0).floor() as usize).clamp(1, 10);
            let tile_width = (ui.available_width() - 6.0 * (columns - 1) as f32) / columns as f32;
            let ready =
                status.state == EngineState::Running || !self.settings.output_device.is_empty();
            egui::Grid::new("soundboard-grid")
                .num_columns(columns)
                .min_col_width(tile_width)
                .max_col_width(tile_width)
                .spacing([6.0, 6.0])
                .show(ui, |ui| {
                    for (index, entry) in library.entries.iter().enumerate() {
                        let playing = (state.playing && state.path.as_ref() == Some(&entry.path))
                            || (monitor.playing && monitor.path.as_ref() == Some(&entry.path));
                        let label = entry
                            .path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .replace('_', " ");
                        let response = ui.add_enabled(
                            ready,
                            egui::Button::new(label)
                                .wrap()
                                .selected(playing)
                                .min_size(egui::vec2(tile_width, 56.0)),
                        );
                        if response.clicked() {
                            selected = Some(entry.path.clone());
                        }
                        response.context_menu(|ui| {
                            if ui
                                .add_enabled(
                                    !self.settings.monitor_device.is_empty()
                                        && self.settings.monitor_device
                                            != self.settings.output_device,
                                    egui::Button::new("Preview in headphones only"),
                                )
                                .clicked()
                            {
                                preview = Some(entry.path.clone());
                                ui.close();
                            }
                        });
                        if (index + 1) % columns == 0 {
                            ui.end_row();
                        }
                    }
                });
        }
        if let Some(path) = selected {
            self.play_soundboard(path, status);
        }
        if let Some(path) = preview {
            self.preview_soundboard(path, status);
        }
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
        ui.small("Right-click a sound to preview it in headphones only. Preview stops any currently playing sound.");
    }

    fn preview_soundboard(&mut self, path: PathBuf, status: &EngineStatusSnapshot) {
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
            &status.output_device
        } else {
            &settings.output_device
        };
        self.soundboard.error = self
            .soundboard
            .player
            .preview(
                path,
                settings.output_host(),
                output.clone(),
                settings.monitor_device.clone(),
            )
            .err()
            .map(|error| error.to_string());
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
