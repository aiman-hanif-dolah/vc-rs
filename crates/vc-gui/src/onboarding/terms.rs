//! Presentation and consent use the same shipped text. Packaging verifies the
//! bootstrapper's license against this copy, so an SDK update cannot silently
//! keep an old consent. Vendor terms remain authoritative at their linked URLs.
use super::*;
use sha2::{Digest, Sha256};

pub(super) const WINDOWS_TERMS: &str =
    include_str!("../../../../scripts/licenses/static/WindowsAppSDK-Onboarding.txt");
pub(super) const COMPONENTS: &str =
    include_str!("../../../../scripts/licenses/static/ONBOARDING-COMPONENTS.md");

pub(super) fn required(settings: &GuiSettings) -> Vec<String> {
    let mut ids = vec!["vc-rs-mit-v1".to_string()];
    if cfg!(feature = "windowsml") || settings.provider.starts_with("windowsml") {
        ids.push(format!(
            "windowsml:{:x}",
            Sha256::digest(format!("{WINDOWS_TERMS}\n{COMPONENTS}"))
        ));
    }
    if cfg!(feature = "tensorrt")
        || cfg!(feature = "cuda")
        || settings.provider.contains("tensorrt")
        || settings.provider == "cuda"
    {
        ids.push(format!("nvidia:{:x}", Sha256::digest(COMPONENTS)));
    }
    ids
}

pub(super) fn accepted(settings: &GuiSettings) -> bool {
    required(settings)
        .iter()
        .all(|id| settings.accepted_terms.contains(id))
}

pub(super) fn view(ui: &mut egui::Ui, settings: &GuiSettings) {
    let lang = settings.language;
    ui.label(lang.text("Review the terms for this app and its processing components. Original license texts are authoritative."));
    egui::CollapsingHeader::new("vc-rs · MIT").show(ui, |ui| {
        ui.label(include_str!("../../../../LICENSE"));
    });
    if cfg!(feature = "windowsml") || settings.provider.starts_with("windowsml") {
        egui::CollapsingHeader::new("Windows App SDK").show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(180.0)
                .show(ui, |ui| {
                    ui.label(WINDOWS_TERMS);
                });
        });
        ui.hyperlink_to(
            "Windows ML Runtime · Microsoft license",
            "https://www.nuget.org/packages/Microsoft.Windows.AI.MachineLearning/2.1.74/License",
        );
        ui.hyperlink_to(
            lang.text("Processing component terms (NVIDIA / Intel / AMD / Qualcomm)"),
            "https://learn.microsoft.com/windows/ai/new-windows-ml/supported-execution-providers",
        );
        ui.label(lang.text("Windows ML may obtain and update hardware-specific components. Their vendor terms also apply. Microsoft components may collect diagnostic data; see the privacy statement."));
        ui.hyperlink_to(
            lang.text("Microsoft privacy statement"),
            "https://privacy.microsoft.com/privacystatement",
        );
    }
    if cfg!(feature = "tensorrt")
        || cfg!(feature = "cuda")
        || settings.provider.contains("tensorrt")
        || settings.provider == "cuda"
    {
        ui.hyperlink_to(
            "NVIDIA TensorRT SDK license",
            "https://docs.nvidia.com/deeplearning/tensorrt/latest/reference/sla.html",
        );
        ui.hyperlink_to(
            "NVIDIA CUDA license",
            "https://docs.nvidia.com/cuda/eula/index.html",
        );
    }
    egui::CollapsingHeader::new(lang.text("Component information")).show(ui, |ui| {
        ui.label(COMPONENTS);
    });
}
