//! Run with each distribution feature set, not just their unified dev build.
use super::*;

fn backend_harness(fixture: Fixture) -> Harness<'static, Fixture> {
    let lang = fixture.app.settings.language;
    let mut h = harness(fixture);
    h.get_by_label(lang.text("Backend Details")).scroll_to_me();
    h.run_steps(8);
    h.get_by_label(lang.text("Backend Details")).click();
    h.run_steps(4);
    h.get_by_label(lang.text("Provider")).scroll_to_me();
    h.run_steps(8);
    h
}

#[test]
fn ui_provider_options_match_build_and_priority_is_always_available() {
    for lang in [text::Language::English, text::Language::Japanese] {
        let mut fixture = ready_main_fixture(lang);
        fixture.app.settings.normalize_gui_managed_settings();
        let mut h = backend_harness(fixture);
        h.get_by_label(lang.text("GPU Priority"));
        h.get_by_label(lang.text("Provider")).click();
        h.run_steps(4);
        for (label, expected) in [
            ("windowsml", cfg!(feature = "windowsml")),
            ("CPU", cfg!(feature = "windowsml")),
            ("windowsml-directml", cfg!(feature = "windowsml")),
            ("tensorrt", cfg!(feature = "tensorrt")),
        ] {
            assert_eq!(h.query_by_label(label).is_some(), expected, "{label}");
        }
        assert!(h.query_by_label("cpu").is_none());
        assert!(h.query_by_label("cuda").is_none());
        assert!(h.query_by_label("windowsml-openvino").is_none());
        let selected = if cfg!(feature = "windowsml") {
            "CPU"
        } else {
            "tensorrt"
        };
        h.get_by_label(selected).click();
        h.run_steps(4);
        assert_eq!(
            h.state().app.settings.provider,
            if cfg!(feature = "windowsml") {
                "windowsml-cpu"
            } else {
                "tensorrt"
            }
        );
        assert_eq!(
            h.query_by_label(lang.text("Detecting CUDA devices..."))
                .is_some(),
            !cfg!(feature = "windowsml")
        );
    }
}

#[cfg(feature = "windowsml")]
#[test]
fn ui_catalog_providers_can_be_selected_without_exposing_cuda_device_controls() {
    for lang in [text::Language::English, text::Language::Japanese] {
        let mut fixture = ready_main_fixture(lang);
        fixture.app.onboarding.effects.catalog_providers = vec![
            Provider::WindowsMlNvTensorRtRtx,
            Provider::WindowsMlOpenVino,
            Provider::WindowsMlOpenVinoCpu,
            Provider::WindowsMlOpenVinoGpu,
            Provider::WindowsMlOpenVinoNpu,
        ];
        let mut h = backend_harness(fixture);
        for label in [
            "windowsml-nvtrtx",
            "windowsml-openvino",
            "windowsml-openvino-cpu",
            "windowsml-openvino-gpu",
            "windowsml-openvino-npu",
            "windowsml",
            "windowsml-directml",
        ] {
            h.get_by_label(lang.text("Provider")).click();
            h.run_steps(4);
            assert!(h.query_by_label("windowsml-qnn").is_none());
            h.get_by_label(label).click();
            h.run_steps(4);
            assert_eq!(h.state().app.settings.provider, label);
            let saved = toml::to_string(&h.state().app.settings).unwrap();
            let mut restored: GuiSettings = toml::from_str(&saved).unwrap();
            restored.normalize_gui_managed_settings();
            assert_eq!(restored.provider, label);
            assert!(h.query_by_label(lang.text("GPU Device")).is_none());
            assert!(h
                .query_by_label(lang.text("Detecting CUDA devices..."))
                .is_none());
            h.get_by_label(lang.text("GPU Priority"));
        }
    }
}

#[cfg(feature = "tensorrt")]
#[test]
fn ui_tensorrt_gpu_discovery_pending_success_missing_and_failure() {
    for lang in [text::Language::English, text::Language::Japanese] {
        let mut fixture = ready_main_fixture(lang);
        fixture.app.settings.provider = "tensorrt".into();
        fixture.app.settings.gpu_device_id = 7;
        let mut h = backend_harness(fixture);
        h.get_by_label(lang.text("Detecting CUDA devices..."));
        h.state().app.gpu_devices.lock().unwrap().devices = Some(vec![
            GpuDevice {
                id: 0,
                display_name: "Test GPU A".into(),
            },
            GpuDevice {
                id: 1,
                display_name: "Test GPU B".into(),
            },
        ]);
        h.run_steps(4);
        h.get_by_value(&format!("{} 7", lang.text("Unavailable: device")));
        assert_eq!(h.state().app.settings.gpu_device_id, 7);
        h.get_by_label(lang.text("GPU Device")).click();
        h.run_steps(4);
        h.get_by_label("1: Test GPU B").click();
        h.run_steps(4);
        assert_eq!(h.state().app.settings.gpu_device_id, 1);
        assert!(h.state().app.dirty_since.is_some());
        h.state().app.gpu_devices.lock().unwrap().devices = Some(vec![]);
        h.run_steps(4);
        h.get_by_value(&format!("{} 1", lang.text("Unavailable: device")));
        *h.state().app.gpu_devices.lock().unwrap() = GpuDeviceDiscovery {
            devices: None,
            error: Some("Test enumeration error".into()),
        };
        h.run_steps(4);
        assert!(h.query_by_label(lang.text("GPU Device")).is_none());
        h.get_by_label(&format!(
            "{}: Test enumeration error",
            lang.text("GPU enumeration failed")
        ));
        assert_eq!(h.state().app.settings.gpu_device_id, 1);
    }
}

#[test]
fn ui_single_backend_defaults_migrate_other_package_and_show_matching_terms() {
    if cfg!(all(feature = "windowsml", feature = "tensorrt")) {
        return;
    }
    let expected = if cfg!(feature = "windowsml") {
        "windowsml"
    } else {
        "tensorrt"
    };
    assert_eq!(GuiSettings::new_user().provider, expected);
    for lang in [text::Language::English, text::Language::Japanese] {
        for old_provider in if cfg!(feature = "windowsml") {
            ["tensorrt", "cuda"]
        } else {
            ["windowsml", "windowsml-openvino"]
        } {
            let mut fixture = ready_main_fixture(lang);
            fixture.app.settings.provider = old_provider.into();
            fixture.app.settings.normalize_gui_managed_settings();
            assert_eq!(fixture.app.settings.provider, expected);
            fixture.app.settings.accepted_terms.clear();
            fixture.app.onboarding.step = Some(Step::Terms);
            let mut h = harness(fixture);
            assert_eq!(
                h.query_by_label("Windows App SDK").is_some(),
                cfg!(feature = "windowsml")
            );
            assert_eq!(
                h.query_by_label("NVIDIA TensorRT SDK license").is_some(),
                cfg!(feature = "tensorrt")
            );
            assert_eq!(
                h.query_by_label("NVIDIA CUDA license").is_some(),
                cfg!(feature = "tensorrt")
            );
            h.get_by_label(lang.text("Agree and continue"))
                .scroll_to_me();
            h.run_steps(8);
            h.get_by_label(lang.text("Agree and continue")).click();
            h.run_steps(4);
            let ids = &h.state().app.settings.accepted_terms;
            assert_eq!(
                ids.iter().any(|id| id.starts_with("windowsml:")),
                cfg!(feature = "windowsml")
            );
            assert_eq!(
                ids.iter().any(|id| id.starts_with("nvidia:")),
                cfg!(feature = "tensorrt")
            );
        }
    }
}

#[cfg(all(feature = "tensorrt", not(feature = "windowsml")))]
#[test]
fn ui_tensorrt_setup_never_requires_windows_ml_preparation() {
    for lang in [text::Language::English, text::Language::Japanese] {
        let mut fixture = ready_main_fixture(lang);
        fixture.app.onboarding.step = Some(Step::Prepare);
        fixture.app.onboarding.runtime_check = None;
        let h = harness(fixture);
        for label in [
            "Checking the runtime…",
            "Prepare processing components",
            text::RUNTIME_LINK,
            text::RUNTIME_RESTART,
        ] {
            assert!(h.query_by_label(lang.text(label)).is_none());
        }
        assert!(!h
            .get_by_label(lang.text(text::OPEN_MAIN))
            .accesskit_node()
            .is_disabled());
        assert!(h.state().app.onboarding.runtime_inspection.is_none());
    }
}
