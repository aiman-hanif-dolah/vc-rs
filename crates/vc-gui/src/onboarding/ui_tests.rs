//! Exercise production widgets with deterministic state, without opening devices,
//! discovering models, or touching the user's persisted settings. Keep these
//! tests below the eframe lifecycle: `VcGui::new` and `maybe_save` perform I/O.
use super::*;
use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
};

struct Fixture {
    app: VcGui,
    status: EngineStatusSnapshot,
    devices: DeviceList,
    setup_owns_screen: bool,
}

fn ready_main_fixture(language: text::Language) -> Fixture {
    let mut fixture = Fixture::new(None, EngineState::Stopped);
    fixture.app.settings.language = language;
    let file = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    fixture.app.settings.model = file.clone();
    fixture.app.settings.embedder = file.clone();
    fixture.app.settings.f0_model = file.clone();
    fixture.app.onboarding.model_check = Some((file, Arc::new(Mutex::new(Some(Ok(()))))));
    fixture.app.onboarding.support_check = Some((
        support_signature(&fixture.app.settings),
        Arc::new(Mutex::new(Some([Ok(()), Ok(())]))),
    ));
    fixture
}

#[test]
fn ui_normal_screen_and_start_do_not_spawn_model_validation() {
    for lang in [text::Language::English, text::Language::Japanese] {
        let mut fixture = ready_main_fixture(lang);
        fixture.app.onboarding.model_check = None;
        fixture.app.onboarding.support_check = None;
        let mut h = harness(fixture);
        h.get_by_label(lang.text("Model")).click();
        h.run_steps(4);
        assert!(h.state().app.onboarding.model_check.is_none());
        assert!(h.state().app.onboarding.support_check.is_none());
        assert!(h.query_by_label(lang.text(text::CHECKING)).is_none());
        h.get_by_label(lang.text(text::START)).click();
        h.run_steps(4);
        assert_eq!(h.state().app.onboarding.effects.start_requests, 1);
        assert!(h.state().app.onboarding.model_check.is_none());
        assert!(h.state().app.onboarding.support_check.is_none());
    }
}

#[test]
fn ui_start_does_not_redirect_for_setup_checks() {
    for lang in [text::Language::English, text::Language::Japanese] {
        for check_state in 0..3 {
            let mut fixture = ready_main_fixture(lang);
            #[cfg(feature = "windowsml")]
            {
                fixture.app.settings.provider = "windowsml-cpu".into();
            }
            fixture.app.onboarding.runtime_check = None;
            fixture.app.onboarding.support_check = match check_state {
                0 => None,
                1 => Some((
                    support_signature(&fixture.app.settings),
                    Arc::new(Mutex::new(None)),
                )),
                _ => Some((
                    support_signature(&fixture.app.settings),
                    Arc::new(Mutex::new(Some([
                        Err("Test validation failure".into()),
                        Ok(()),
                    ]))),
                )),
            };
            if check_state == 0 {
                fixture.app.settings.accepted_terms.clear();
            }
            let mut h = harness(fixture);
            h.get_by_label(lang.text(text::START)).click();
            h.run_steps(4);
            assert_eq!(h.state().app.onboarding.effects.start_requests, 1);
            assert!(h.state().app.onboarding.step.is_none());
            assert!(!h.state().setup_owns_screen);
        }
    }
}

#[test]
fn ui_normal_transport_pending_and_passthrough() {
    for lang in [text::Language::English, text::Language::Japanese] {
        let mut h = harness(ready_main_fixture(lang));
        assert!(h.query_by_label(lang.text(text::STOP)).is_none());
        h.get_by_label(lang.text(text::START)).click();
        h.run_steps(4);
        assert_eq!(h.state().app.onboarding.effects.start_requests, 1);
        assert!(h
            .get_by_label(lang.text(text::START))
            .accesskit_node()
            .is_disabled());
        h.state_mut().status.state = EngineState::Running;
        h.state_mut().status.session_revision = 1;
        h.state_mut().status.passthrough_live_switchable = true;
        h.run_steps(4);
        h.get_by_label(lang.text("Restart"));
        h.state_mut().app.settings.input_gain = 2.0;
        h.state_mut().app.settings.pitch_shift = 3.0;
        h.run_steps(4);
        assert!(h
            .query_by_label(lang.text("Unapplied changes — Restart to apply."))
            .is_none());
        h.state_mut().app.settings.extra_convert_ms += 10;
        h.run_steps(4);
        h.get_by_label(lang.text("Unapplied changes — Restart to apply."));
        h.get_by_label(lang.text("Passthrough")).click();
        h.run_steps(4);
        assert!(h
            .query_all_by_label(lang.text(text::PITCH))
            .all(|node| node.accesskit_node().is_disabled()));
        h.get_by_label(lang.text("Restart")).click();
        h.run_steps(4);
        assert_eq!(h.state().app.onboarding.effects.start_requests, 2);
        h.get_by_label(lang.text(text::STOP)).click();
        h.run_steps(4);
        h.state_mut().status.state = EngineState::Stopped;
        h.run_steps(4);
        assert!(!h
            .get_by_label(lang.text(text::START))
            .accesskit_node()
            .is_disabled());
    }
}

#[test]
fn ui_support_sources_preserve_custom_path_and_download_only_selected_role() {
    for lang in [text::Language::English, text::Language::Japanese] {
        let mut h = harness(ready_main_fixture(lang));
        h.get_by_label(lang.text("Model")).click();
        h.run_steps(4);
        let original = h.state().app.settings.embedder.clone();
        // Combo controls have distinct labels even though both offer Custom.
        h.get_by_label(lang.text("Embedder")).click();
        h.run_steps(2);
        h.get_by_label("ContentVec").click();
        h.run_steps(4);
        assert_eq!(h.state().app.settings.support_custom_paths[0], original);
        assert_eq!(h.state().app.settings.support_custom_mode[0], "downloaded");
        h.get_by_label(lang.text("Embedder")).click();
        h.run_steps(2);
        h.get_by_label(lang.text("Custom")).click();
        h.run_steps(4);
        assert_eq!(h.state().app.settings.embedder, original);
    }
}

#[test]
fn ui_failed_start_can_retry_and_language_is_not_a_restart_change() {
    let mut fixture = ready_main_fixture(text::Language::English);
    fixture.app.onboarding.effects.start_error = Some("Test start failure".into());
    let mut h = harness(fixture);
    h.get_by_label(text::START).click();
    h.run_steps(4);
    assert!(h.state().app.onboarding.normal.requested.is_none());
    assert!(!h.get_by_label(text::START).accesskit_node().is_disabled());
    h.state_mut().app.onboarding.effects.start_error = None;
    h.get_by_label(text::START).click();
    h.run_steps(4);
    h.state_mut().status.state = EngineState::Running;
    h.state_mut().status.session_revision = 1;
    h.run_steps(4);
    h.state_mut().app.settings.language = text::Language::Japanese;
    h.run_steps(4);
    assert!(h
        .query_by_label(text::Language::Japanese.text("Unapplied changes — Restart to apply."))
        .is_none());
    assert_eq!(h.state().app.onboarding.effects.start_requests, 2);
}

#[cfg(feature = "ui-snapshots")]
#[test]
#[ignore = "Render the normal screen without devices, inference, or settings writes"]
fn render_normal_pages() {
    let destination = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/normal-ui");
    fs::create_dir_all(&destination).unwrap();
    for lang in [text::Language::English, text::Language::Japanese] {
        for width in [880.0, 520.0] {
            let mut h = harness_sized(ready_main_fixture(lang), egui::vec2(width, 680.0));
            install_system_japanese_font(&h.ctx);
            h.run_steps(4);
            h.render()
                .unwrap()
                .save(destination.join(format!("{lang:?}-{width}.png")))
                .unwrap();
            h.get_by_label(lang.text("Backend Details")).click();
            h.run_steps(4);
            h.render()
                .unwrap()
                .save(destination.join(format!("{lang:?}-{width}-expanded.png")))
                .unwrap();
        }
    }
}

impl Fixture {
    fn new(step: Option<Step>, state: EngineState) -> Self {
        let mut settings = GuiSettings::default();
        settings.accepted_terms = terms::required(&settings);
        let mut onboarding = Onboarding::new(&settings);
        onboarding.step = step;
        onboarding.runtime_check = Some((
            settings.provider.clone(),
            Arc::new(Mutex::new(Some(Ok(())))),
        ));
        onboarding.support_check = Some((
            support_signature(&settings),
            Arc::new(Mutex::new(Some([
                Err("Missing test support files".into()),
                Err("Missing test support files".into()),
            ]))),
        ));
        Self {
            app: VcGui {
                // The idle controller only waits for commands. Never click Start
                // with valid models or Refresh in this fixture: those need a
                // separate integration test with explicit device authorization.
                controller: EngineController::new(settings.live()),
                settings,
                dirty_since: None,
                ui_error: None,
                telemetry: TelemetrySnapshot::default(),
                telemetry_updated_at: Instant::now(),
                applied_chunk_ms: None,
                gpu_devices: Arc::new(Mutex::new(GpuDeviceDiscovery::default())),
                pth_convert: None,
                model_download: None,
                onboarding,
            },
            status: EngineStatusSnapshot {
                state,
                ..Default::default()
            },
            devices: DeviceList {
                inputs: vec!["Test microphone".into()],
                outputs: vec!["Test headphones".into()],
                error: None,
            },
            setup_owns_screen: false,
        }
    }
}

fn harness(fixture: Fixture) -> Harness<'static, Fixture> {
    harness_sized(fixture, egui::vec2(880.0, 680.0))
}

fn harness_sized(fixture: Fixture, size: egui::Vec2) -> Harness<'static, Fixture> {
    let builder = Harness::builder();
    #[cfg(feature = "ui-snapshots")]
    let builder = builder.renderer(egui_kittest::LazyRenderer::new(
        super::ui_renderer::WgpuTestRenderer::new,
    ));
    builder.with_size(size).build_ui_state(
        |ui, fixture: &mut Fixture| {
            egui::Frame::new()
                .fill(ui.visuals().panel_fill)
                .inner_margin(20)
                .show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui_text::set_language(ui.ctx(), fixture.app.settings.language);
                        fixture.setup_owns_screen =
                            fixture
                                .app
                                .onboarding_ui(ui, &fixture.status, &fixture.devices);
                        if !fixture.setup_owns_screen {
                            fixture.app.basic_ui(ui, &fixture.status, &fixture.devices);
                        }
                        fixture.app.pth_convert_window(ui.ctx());
                    });
                });
        },
        fixture,
    )
}

#[test]
fn ui_setup_cannot_advance_without_a_voice() {
    for language in [ui_text::Language::English, ui_text::Language::Japanese] {
        let mut fixture = Fixture::new(Some(Step::Voice), EngineState::Stopped);
        fixture.app.settings.language = language;
        let mut h = harness(fixture);
        h.get_by_label(language.text(text::VOICE_TITLE));
        assert!(h
            .get_by_label(language.text(text::NEXT))
            .accesskit_node()
            .is_disabled());
        h.get_by_label(language.text(text::NEXT)).click();
        h.run_steps(4);
        assert_eq!(h.state().app.onboarding.step, Some(Step::Voice));
        assert!(!h.state().app.settings.setup_completed);
        assert!(h.state().setup_owns_screen);
    }
}

#[test]
fn ui_support_batch_includes_gtcrn_only_when_built_without_enabling_it() {
    let mut h = harness(Fixture::new(Some(Step::Prepare), EngineState::Stopped));
    let denoiser = h.state().app.settings.denoiser.clone();
    h.get_by_label(text::DOWNLOAD).click();
    h.run_steps(4);
    let mut expected = vec![0, 1];
    if cfg!(feature = "gtcrn") {
        expected.push(model_setup::GTCRN_INDEX);
        assert!(h
            .state()
            .app
            .settings
            .accepted_terms
            .iter()
            .any(|id| id == "gtcrn-mit-502ebfab"));
    }
    assert_eq!(h.state().app.onboarding.effects.download_indices, expected);
    assert_eq!(h.state().app.settings.denoiser, denoiser);
}

#[cfg(feature = "windowsml")]
#[test]
fn ui_legacy_cpu_uses_the_single_windows_ml_cpu_option() {
    let mut fixture = Fixture::new(None, EngineState::Stopped);
    fixture.app.settings.provider = "cpu".into();
    fixture.app.settings.chunk_ms = 230;
    fixture.app.settings.normalize_gui_managed_settings();
    assert_eq!(fixture.app.settings.provider, "windowsml-cpu");
    assert_eq!(fixture.app.settings.chunk_ms, 230);
    assert!(!gui_provider_visible(vc_core::Provider::Cpu));
    assert!(gui_provider_visible(vc_core::Provider::WindowsMlCpu));
    assert_eq!(gui_provider_label("windowsml-cpu"), "CPU");
}

#[test]
fn ui_language_terms_audio_flow_does_not_start_output() {
    for language in [text::Language::English, text::Language::Japanese] {
        let mut fixture = Fixture::new(Some(Step::Language), EngineState::Stopped);
        fixture.app.settings.accepted_terms.clear();
        fixture.app.settings.language = language;
        let mut h = harness(fixture);
        h.get_by_label(language.text("Next")).click();
        h.run_steps(4);
        assert_eq!(h.state().app.onboarding.step, Some(Step::Terms));
        assert!(h.state().app.onboarding.effects.device_requests.is_empty());
        h.get_by_label(language.text(text::BACK)).click();
        h.run_steps(4);
        assert_eq!(h.state().app.onboarding.step, Some(Step::Language));
        assert!(h.state().app.settings.accepted_terms.is_empty());
        assert_eq!(
            h.state().app.settings.tutorial.as_ref().unwrap().step,
            Some(Step::Language)
        );
        h.get_by_label(language.text("Next")).click();
        h.run_steps(4);
        assert_eq!(h.state().app.onboarding.step, Some(Step::Terms));
        h.get_by_label(language.text("Agree and continue")).click();
        h.run_steps(4);
        assert_eq!(h.state().app.onboarding.step, Some(Step::Audio));
        assert!(terms::accepted(&h.state().app.settings));
        assert_eq!(
            h.state().app.onboarding.effects.device_requests,
            [vc_app::TestOutput::Silent]
        );
        h.get_by_label(language.text("Continue with these devices"))
            .click();
        h.run_steps(4);
        assert_eq!(h.state().app.onboarding.step, Some(Step::Voice));
    }
}

#[test]
fn ui_save_failure_preserves_page_and_does_not_accept_terms() {
    let mut fixture = Fixture::new(Some(Step::Terms), EngineState::Stopped);
    fixture.app.settings.accepted_terms.clear();
    fixture.app.onboarding.effects.save_error = Some("Read-only test settings".into());
    let mut h = harness(fixture);
    h.get_by_label("Agree and continue").click();
    h.run_steps(4);
    assert_eq!(h.state().app.onboarding.step, Some(Step::Terms));
    assert!(h.state().app.settings.accepted_terms.is_empty());
    assert!(h.state().app.onboarding.effects.saved.is_empty());
    assert!(h.state().app.ui_error.is_some());
}

#[test]
fn ui_skip_save_failure_stays_on_audio_page() {
    let mut fixture = Fixture::new(Some(Step::Audio), EngineState::Stopped);
    fixture.app.onboarding.effects.save_error = Some("Test save failure".into());
    let mut h = harness(fixture);
    h.get_by_label(text::SKIP).click();
    h.run_steps(4);
    assert_eq!(h.state().app.onboarding.step, Some(Step::Audio));
    assert!(!h.state().app.settings.setup_skipped);
}

#[test]
fn ui_device_change_silences_monitor_and_uses_new_device() {
    let mut h = harness(Fixture::new(Some(Step::Audio), EngineState::Stopped));
    h.get_by_label("Hear my voice").click();
    h.run_steps(4);
    assert_eq!(
        h.state().app.onboarding.effects.device_requests.last(),
        Some(&vc_app::TestOutput::Monitor)
    );
    h.state_mut().app.settings.output_device = "Changed output".into();
    h.run_steps(4);
    assert_eq!(
        h.state().app.onboarding.effects.device_requests.last(),
        Some(&vc_app::TestOutput::Silent)
    );
}

#[cfg(feature = "ui-snapshots")]
#[test]
#[ignore = "Renders setup pages to target/onboarding-ui for visual review"]
fn render_tutorial_pages() {
    let destination = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/onboarding-ui");
    fs::create_dir_all(&destination).unwrap();
    for language in [text::Language::English, text::Language::Japanese] {
        for step in [
            Step::Language,
            Step::Terms,
            Step::Audio,
            Step::Voice,
            Step::Prepare,
        ] {
            let mut fixture = Fixture::new(Some(step), EngineState::Stopped);
            fixture.app.settings.language = language;
            let mut h = harness(fixture);
            install_system_japanese_font(&h.ctx);
            h.run_steps(4);
            h.render()
                .unwrap()
                .save(destination.join(format!("{language:?}-{step:?}.png")))
                .unwrap();
        }
    }
}

#[test]
fn ui_prepared_voice_waits_for_continue_on_preparation() {
    for language in [ui_text::Language::English, ui_text::Language::Japanese] {
        let mut fixture = Fixture::new(Some(Step::Voice), EngineState::Stopped);
        fixture.app.settings.language = language;
        // Supply a cached successful validation and an existing file solely for
        // availability checks. This fixture must never load or parse this file.
        let path = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        fixture.app.settings.model = path.clone();
        fixture.app.settings.embedder = path.clone();
        fixture.app.settings.f0_model = path.clone();
        fixture.app.onboarding.support_check = Some((
            support_signature(&fixture.app.settings),
            Arc::new(Mutex::new(Some([Ok(()), Ok(())]))),
        ));
        fixture.app.onboarding.model_check = Some((path, Arc::new(Mutex::new(Some(Ok(()))))));
        let mut h = harness(fixture);
        h.get_by_label(language.text(text::NEXT)).click();
        h.run_steps(4);
        assert_eq!(h.state().app.onboarding.step, Some(Step::Prepare));
        h.get_by_label(&format!("ContentVec: {}", language.text("Ready")));
        h.get_by_label(&format!("RMVPE: {}", language.text("Ready")));
        assert_eq!(h.state().app.onboarding.effects.download_requests, 0);
        h.get_by_label(language.text(text::OPEN_MAIN)).click();
        h.run_steps(4);
        assert_eq!(h.state().app.onboarding.step, None);
        assert!(h.state().app.settings.setup_completed);
        assert_eq!(h.state().status.state, EngineState::Stopped);
        assert!(h.state().app.onboarding.effects.device_requests.is_empty());
        assert_eq!(h.state().app.settings.output_gain, 1.0);
        h.get_by_label(language.text(text::SETUP)).click();
        h.run_steps(4);
        assert_eq!(h.state().app.onboarding.step, Some(Step::Audio));
    }
}

#[test]
fn ui_valid_voice_still_requires_shared_files() {
    let mut fixture = Fixture::new(Some(Step::Voice), EngineState::Stopped);
    let path = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    fixture.app.settings.model = path.clone();
    fixture.app.onboarding.model_check = Some((path, Arc::new(Mutex::new(Some(Ok(()))))));
    let mut h = harness(fixture);
    h.get_by_label(text::NEXT).click();
    h.run_steps(4);
    assert_eq!(h.state().app.onboarding.step, Some(Step::Prepare));
    h.get_by_label(text::DOWNLOAD);
}

#[test]
fn ui_preparation_downloads_missing_files_and_waits_after_completion() {
    for language in [text::Language::English, text::Language::Japanese] {
        for missing in 0..2 {
            let mut fixture = Fixture::new(Some(Step::Prepare), EngineState::Stopped);
            fixture.app.settings.language = language;
            fixture.app.settings.model = std::env::current_exe()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let check = fixture
                .app
                .onboarding
                .support_check
                .as_ref()
                .unwrap()
                .1
                .clone();
            *check.lock().unwrap() = None;
            let mut h = harness(fixture);
            assert!(h.query_by_label(language.text(text::DOWNLOAD)).is_none());
            assert!(h
                .query_by_label(language.text(text::DOWNLOAD_INFO))
                .is_none());
            assert!(h
                .get_by_label(language.text(text::OPEN_MAIN))
                .accesskit_node()
                .is_disabled());
            let mut results = [Ok(()), Ok(())];
            results[missing] = Err("Missing or corrupt file".into());
            *check.lock().unwrap() = Some(results);
            h.run_steps(4);
            assert!(h
                .get_by_label(language.text(text::OPEN_MAIN))
                .accesskit_node()
                .is_disabled());
            h.get_by_label(language.text(text::DOWNLOAD)).click();
            h.run_steps(4);
            let mut expected = vec![missing];
            if cfg!(feature = "gtcrn") {
                expected.push(model_setup::GTCRN_INDEX);
            }
            assert_eq!(h.state().app.onboarding.effects.download_indices, expected);
            *check.lock().unwrap() = Some([Ok(()), Ok(())]);
            h.run_steps(4);
            assert!(h.query_by_label(language.text(text::DOWNLOAD)).is_none());
            assert_eq!(h.state().app.onboarding.step, Some(Step::Prepare));
            assert!(!h
                .get_by_label(language.text(text::OPEN_MAIN))
                .accesskit_node()
                .is_disabled());
            h.state_mut().app.onboarding.effects.save_error = Some("Test save failure".into());
            h.get_by_label(language.text(text::OPEN_MAIN)).click();
            h.run_steps(4);
            assert_eq!(h.state().app.onboarding.step, Some(Step::Prepare));
            h.state_mut().app.onboarding.effects.save_error = None;
            h.get_by_label(language.text(text::OPEN_MAIN)).click();
            h.run_steps(4);
            assert_eq!(h.state().app.onboarding.step, None);
        }
    }
}

#[cfg(feature = "windowsml")]
#[test]
fn ui_runtime_download_prompt_waits_for_inspection() {
    for language in [text::Language::English, text::Language::Japanese] {
        for result in [Ok(true), Ok(false), Err("Test inspection failure".into())] {
            let mut fixture = Fixture::new(Some(Step::Prepare), EngineState::Stopped);
            fixture.app.settings.language = language;
            fixture.app.settings.provider = "windowsml".into();
            fixture.app.onboarding.runtime_check = None;
            let inspection = Arc::new(Mutex::new(None));
            fixture.app.onboarding.runtime_inspection =
                Some(("windowsml".into(), inspection.clone()));
            let mut h = harness(fixture);
            let label = language.text("Prepare processing components");
            assert!(h.query_by_label(label).is_none());
            h.get_by_label(language.text("Checking the runtime…"));
            *inspection.lock().unwrap() = Some(result.clone());
            h.run_steps(4);
            assert_eq!(h.query_by_label(label).is_some(), result == Ok(true));
            assert_eq!(h.state().app.onboarding.step, Some(Step::Prepare));
            if result.is_err() {
                h.get_by_label(language.text(text::RUNTIME_HELP));
                h.get_by_label(language.text(text::RUNTIME_LINK));
                h.get_by_label(language.text(text::RUNTIME_RESTART));
                h.get_by_label(language.text("Runtime details"))
                    .scroll_to_me();
                h.run_steps(4);
                h.get_by_label(language.text("Runtime details")).click();
                h.run_steps(4);
                h.get_by_label("Test inspection failure");
                h.get_by_label(language.text("Retry runtime check"))
                    .scroll_to_me();
                h.run_steps(4);
                h.get_by_label(language.text("Retry runtime check")).click();
                h.run_steps(4);
                assert!(h
                    .query_by_label(language.text(text::RUNTIME_LINK))
                    .is_none());
                assert_eq!(h.state().app.onboarding.step, Some(Step::Prepare));
            } else {
                assert!(h
                    .query_by_label(language.text(text::RUNTIME_LINK))
                    .is_none());
            }
        }
    }
}

#[test]
fn ui_skip_opens_normal_screen_and_persists_without_claiming_preview() {
    for language in [ui_text::Language::English, ui_text::Language::Japanese] {
        for step in [Step::Audio, Step::Voice, Step::Prepare] {
            let mut fixture = Fixture::new(Some(step), EngineState::Stopped);
            fixture.app.settings.language = language;
            let mut h = harness(fixture);
            let skip = match step {
                Step::Voice => "Choose a voice later",
                _ => text::SKIP,
            };
            h.get_by_label(language.text(skip)).click();
            h.run_steps(4);
            assert!(!h.state().setup_owns_screen);
            assert_eq!(h.state().app.onboarding.step, None);
            assert!(h.state().app.settings.setup_skipped);
            assert!(!h.state().app.settings.setup_completed);
            assert!(h.state().app.dirty_since.is_none());
            assert!(!h.state().app.onboarding.effects.saved.is_empty());
            h.get_by_label(language.text(text::START));
            let saved = toml::to_string(&h.state().app.settings).unwrap();
            let restored: GuiSettings = toml::from_str(&saved).unwrap();
            assert_eq!(Onboarding::new(&restored).step, None);
            h.get_by_label(language.text(text::SETUP)).click();
            h.run_steps(4);
            assert_eq!(h.state().app.onboarding.step, Some(Step::Audio));
        }
    }
}

#[test]
fn ui_busy_engine_disables_start() {
    for state in [EngineState::Starting, EngineState::Stopping] {
        let mut h = harness(Fixture::new(None, state));
        assert!(h.get_by_label(text::START).accesskit_node().is_disabled());
        h.get_by_label(text::START).click();
        h.run_steps(4);
        // An enabled Start would reach model validation and set an error.
        assert!(h.state().app.ui_error.is_none());
        assert!(h.state().app.applied_chunk_ms.is_none());
    }
}

#[test]
fn ui_start_with_missing_models_reports_validation_error() {
    let mut h = harness(Fixture::new(None, EngineState::Stopped));
    h.get_by_label(text::START).click();
    h.run_steps(4);
    assert_eq!(
        h.state().app.ui_error.as_deref(),
        Some("Choose a voice model and prepare ContentVec / RMVPE in Setup.")
    );
    assert!(h.state().app.applied_chunk_ms.is_none());
}

#[test]
fn ui_setup_button_returns_to_device_test() {
    let fixture = Fixture::new(None, EngineState::Stopped);
    let mut h = harness(fixture);
    h.get_by_label(text::SETUP).click();
    h.run_steps(4);
    h.get_by_label("Check your microphone and headphones");
    assert!(h.state().setup_owns_screen);
}

#[cfg(feature = "windowsml")]
#[test]
fn ui_failed_conversion_can_retry_then_cancel_without_changing_model() {
    let mut fixture = Fixture::new(Some(Step::Voice), EngineState::Stopped);
    let convert = PthConvert::new(PathBuf::from("ui-test-voice.pth"));
    *convert.state.lock().unwrap() = PthConvertState::Failed {
        error: "Test conversion failure".into(),
    };
    fixture.app.pth_convert = Some(convert);
    let mut h = harness(fixture);
    h.run_steps(4);
    h.get_by_label("Test conversion failure");
    h.get_by_label("Retry").click();
    h.run_steps(4);
    assert!(matches!(
        *h.state()
            .app
            .pth_convert
            .as_ref()
            .unwrap()
            .state
            .lock()
            .unwrap(),
        PthConvertState::Configuring
    ));
    h.get_by_label("Cancel").click();
    h.run_steps(4);
    assert!(h.state().app.pth_convert.is_none());
    assert!(h.state().app.settings.model.is_empty());
    assert!(h.state().app.dirty_since.is_none());
}
