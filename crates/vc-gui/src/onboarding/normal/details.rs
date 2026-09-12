use super::*;

type ModelResult = Result<ModelDetails, String>;

#[derive(Default)]
pub(super) struct ModelDetailsState {
    path: String,
    result: Option<Arc<Mutex<Option<ModelResult>>>>,
}

struct ModelDetails {
    bytes: u64,
    sample_rate: Option<u32>,
    version: Option<String>,
    speakers: Option<Vec<(String, String)>>,
    descriptions: Vec<(&'static str, String)>,
}

fn read_details(path: &str) -> ModelResult {
    let bytes = fs::metadata(path).map_err(|e| e.to_string())?.len();
    let metadata =
        vc_core::model_rvc::read_model_metadata(Path::new(path)).map_err(|e| format!("{e:#}"))?;
    Ok(details_from_metadata(bytes, metadata))
}

fn details_from_metadata(bytes: u64, info: vc_core::model_rvc::ModelMetadata) -> ModelDetails {
    let sample_rate = info.sample_rate;
    let metadata = info.properties;
    let value = |key: &str| metadata.iter().find(|(k, _)| k == key).map(|(_, v)| v);
    let json = value("metadata").and_then(|v| serde_json::from_str::<serde_json::Value>(v).ok());
    let version = value("rvc.model_version")
        .cloned()
        .or_else(|| json.as_ref()?.get("version")?.as_str().map(str::to_owned));
    let source =
        value("vc-rs.source").and_then(|v| serde_json::from_str::<serde_json::Value>(v).ok());
    let export =
        value("vc-rs.export").and_then(|v| serde_json::from_str::<serde_json::Value>(v).ok());
    let mut descriptions = Vec::new();
    // Only render known schema fields. Unstructured provenance remains in the
    // ONNX file, without turning arbitrary metadata keys into GUI controls.
    if let Some(source) = source
        .as_ref()
        .filter(|v| v["schema_version"] == 1)
        .and_then(|v| v.get("fields"))
    {
        for (key, label) in [
            ("model_name", "Model name"),
            ("author", "Model author"),
            ("info", "Model notes"),
            ("description", "Model description"),
            ("epoch", "Training epoch"),
            ("step", "Training step"),
            ("creation_date", "Model creation date"),
            ("embedder_model", "Training embedder"),
            ("vocoder", "Source vocoder"),
            ("license", "Model license"),
            ("terms", "Model terms"),
            ("producer_name", "Source tool"),
            ("producer_version", "Source tool version"),
        ] {
            if let Some(v) = source.get(key) {
                let text = match v {
                    serde_json::Value::String(v) if !v.trim().is_empty() => Some(v.clone()),
                    serde_json::Value::Number(v) => Some(v.to_string()),
                    _ => None,
                };
                if let Some(text) = text {
                    descriptions.push((label, text));
                }
            }
        }
    }
    if let Some(export) = export.as_ref().filter(|v| v["schema_version"] == 1) {
        for (key, label) in [
            ("producer_name", "Export tool"),
            ("producer_version", "Export tool version"),
            ("format", "Export format"),
        ] {
            if let Some(v) = export
                .get(key)
                .and_then(|v| v.as_str())
                .filter(|v| !v.is_empty())
            {
                descriptions.push((label, v.into()));
            }
        }
    }
    let speakers = json
        .as_ref()
        .and_then(|v| v.get("speakers"))
        .and_then(|v| {
            v.as_object()
                .map(|v| {
                    v.iter()
                        .filter_map(|(id, name)| Some((id.clone(), name.as_str()?.to_owned())))
                        .collect::<Vec<_>>()
                })
                .or_else(|| {
                    v.as_array().map(|v| {
                        v.iter()
                            .enumerate()
                            .filter_map(|(id, name)| {
                                Some((id.to_string(), name.as_str()?.to_owned()))
                            })
                            .collect::<Vec<_>>()
                    })
                })
        })
        .filter(|speakers| !speakers.is_empty());
    ModelDetails {
        bytes,
        sample_rate,
        version,
        speakers,
        descriptions,
    }
}

impl VcGui {
    pub(super) fn model_details_ui(&mut self, ui: &mut egui::Ui, status: &EngineStatusSnapshot) {
        let lang = self.settings.language;
        ui.strong(lang.text("Model information"));
        ui.small(lang.text("Selected file"));
        if self.settings.model.is_empty() {
            ui.label(lang.text("No model selected."));
            return;
        }
        ui.label(
            Path::new(&self.settings.model)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
        );
        ui.label(&self.settings.model);
        if status.state == EngineState::Running {
            if let Some(applied) = &self.onboarding.normal.applied {
                if applied.model != self.settings.model {
                    ui.label(format!("{}: {}", lang.text("Running model"), applied.model));
                }
            }
        }
        if is_pth_path(&self.settings.model) {
            ui.small(lang.text("Model information is available after ONNX conversion."));
            return;
        }
        let state = &mut self.onboarding.normal.details;
        let loading = state
            .result
            .as_ref()
            .is_some_and(|r| r.lock().unwrap().is_none());
        let refresh = ui
            .add_enabled(
                !loading,
                egui::Button::new(lang.text("Refresh information")).small(),
            )
            .clicked();
        // Read once per selection, with an explicit refresh for files replaced in place.
        // Each worker owns its result; an old selection cannot overwrite a newer one.
        if state.path != self.settings.model || state.result.is_none() || refresh {
            state.path = self.settings.model.clone();
            let result = Arc::new(Mutex::new(None));
            let worker = result.clone();
            let path = state.path.clone();
            let ctx = ui.ctx().clone();
            if let Err(error) = std::thread::Builder::new()
                .name("vc-model-info".into())
                .spawn(move || {
                    let details = read_details(&path);
                    *worker.lock().unwrap() = Some(details);
                    ctx.request_repaint();
                })
            {
                *result.lock().unwrap() = Some(Err(error.to_string()));
            }
            state.result = Some(result);
        }
        let result = state.result.as_ref().unwrap().lock().unwrap();
        match result.as_ref() {
            None => {
                ui.spinner();
            }
            Some(Err(error)) => {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
            Some(Ok(info)) => {
                ui.label(format!(
                    "{}: {:.1} MiB",
                    lang.text("File size"),
                    info.bytes as f64 / 1_048_576.0
                ));
                ui.label(format!(
                    "{}: {}",
                    lang.text("Sample rate"),
                    rate_label(info.sample_rate, lang)
                ));
                ui.label(format!(
                    "{}: {}",
                    lang.text("Model version"),
                    info.version.as_deref().unwrap_or(lang.text("Unknown"))
                ));
                // Speaker names are optional export metadata, not a guessed
                // count from the selected Speaker ID or a default single voice.
                if let Some(speakers) = &info.speakers {
                    ui.label(format!(
                        "{}: {}",
                        lang.text("Speaker count"),
                        speakers.len()
                    ));
                    for (id, name) in speakers {
                        ui.label(format!("{} {id}: {name}", lang.text("Speaker ID")));
                    }
                }
                for (label, value) in &info.descriptions {
                    ui.label(format!("{}: {value}", lang.text(label)));
                }
            }
        }
    }
}

fn rate_label(rate: Option<u32>, lang: text::Language) -> String {
    rate.filter(|rate| *rate > 0)
        .map(|rate| format!("{rate} Hz"))
        .unwrap_or_else(|| lang.text("Unknown").into())
}

pub(super) fn audio_details_ui(
    ui: &mut egui::Ui,
    lang: text::Language,
    status: &EngineStatusSnapshot,
) {
    ui.strong(lang.text("Active audio devices"));
    if status.state != EngineState::Running {
        ui.small(lang.text("Device information is available while running."));
        return;
    }
    // Use the active snapshot, never the pending device selection or its defaults.
    for (role, name, rate, format) in [
        (
            "Audio input",
            &status.input_device,
            status.input_sample_rate,
            &status.input_format,
        ),
        (
            "Audio output",
            &status.output_device,
            status.output_sample_rate,
            &status.output_format,
        ),
    ] {
        ui.label(format!(
            "{}: {}",
            lang.text(role),
            if name.is_empty() {
                lang.text("Unknown")
            } else {
                name
            }
        ));
        ui.small(format!(
            "{} · {}",
            rate_label(Some(rate), lang),
            if format.is_empty() {
                lang.text("Unknown")
            } else {
                format
            }
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_and_export_identity_remain_distinct() {
        let info = details_from_metadata(42, vc_core::model_rvc::ModelMetadata {
            sample_rate: Some(48_000),
            properties: vec![
                ("vc-rs.source".into(), r#"{"schema_version":1,"fields":{"info":"1500epoch","producer_name":"training-tool","author":"","merge_recipe":{"schema_version":1}}}"#.into()),
                ("vc-rs.export".into(), r#"{"schema_version":1,"producer_name":"vc-convert","format":"webui"}"#.into()),
            ],
        });
        assert!(info
            .descriptions
            .contains(&("Source tool", "training-tool".into())));
        assert!(info
            .descriptions
            .contains(&("Export tool", "vc-convert".into())));
        assert!(info
            .descriptions
            .contains(&("Model notes", "1500epoch".into())));
        assert!(!info
            .descriptions
            .iter()
            .any(|(label, _)| *label == "Model author"));
        assert!(info.speakers.is_none());
    }

    #[test]
    fn model_details_keep_missing_metadata_unknown_and_prefer_explicit_version() {
        let info = details_from_metadata(
            42,
            vc_core::model_rvc::ModelMetadata {
                sample_rate: Some(40_000),
                properties: vec![
                    ("rvc.model_version".into(), "v2".into()),
                    (
                        "metadata".into(),
                        r#"{"version":"v1","speakers":{"0":"A","1":"B"}}"#.into(),
                    ),
                ],
            },
        );
        assert_eq!(info.sample_rate, Some(40_000));
        assert_eq!(info.version.as_deref(), Some("v2"));
        assert_eq!(
            info.speakers,
            Some(vec![("0".into(), "A".into()), ("1".into(), "B".into())])
        );
        for properties in [
            vec![],
            vec![("metadata".into(), "invalid JSON".into())],
            vec![("metadata".into(), r#"{"speakers":{}}"#.into())],
            vec![("metadata".into(), r#"{"speakers":[]}"#.into())],
            vec![("metadata".into(), r#"{"speakers":{"0":null}}"#.into())],
        ] {
            let info = details_from_metadata(
                42,
                vc_core::model_rvc::ModelMetadata {
                    sample_rate: None,
                    properties,
                },
            );
            assert!(info.sample_rate.is_none());
            assert!(info.version.is_none());
            assert!(info.speakers.is_none());
        }
    }

    #[test]
    fn audio_information_hides_stale_session_formats_when_stopped() {
        use egui_kittest::{kittest::Queryable, Harness};
        let mut status = EngineStatusSnapshot {
            state: EngineState::Running,
            input_device: "Actual microphone".into(),
            output_device: "Actual speakers".into(),
            input_sample_rate: 48_000,
            output_sample_rate: 44_100,
            input_format: "2 ch · 32-bit float".into(),
            output_format: "2 ch · 16-bit PCM".into(),
            ..Default::default()
        };
        {
            let h = Harness::new_ui(|ui| audio_details_ui(ui, text::Language::English, &status));
            assert!(h.query_by_label("48000 Hz · 2 ch · 32-bit float").is_some());
            assert!(h.query_by_label("44100 Hz · 2 ch · 16-bit PCM").is_some());
        }
        status.state = EngineState::Stopped;
        let h = Harness::new_ui(|ui| audio_details_ui(ui, text::Language::English, &status));
        assert!(h
            .query_by_label("Device information is available while running.")
            .is_some());
        assert!(h.query_by_label("48000 Hz · 2 ch · 32-bit float").is_none());
    }
}
