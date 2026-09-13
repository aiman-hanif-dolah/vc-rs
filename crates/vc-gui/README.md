# GUI tests

Run the distribution UI matrix with `just test-gui-variants` (also included in
`just test-ci` and CI). It runs separate package-scoped Cargo commands with
`--no-default-features --features windowsml,rnnoise,gtcrn` and
`--no-default-features --features tensorrt,rnnoise,gtcrn` to avoid feature unification.
Native TensorRT is disabled: these checks validate UI behavior, not inference.

`src/onboarding/backend_tests.rs` checks provider choices, injected Windows ML
catalog entries, GPU discovery states, shared GPU priority controls, package
defaults/settings migration and backend-specific consent. Windows ML preparation
and the absence of its prompts in TensorRT are checked separately. The common
`.pth` conversion dialog tests run in both packages. All interaction cases cover
English/Japanese except the existing conversion retry/cancel test.

Run the headless egui interaction tests from the repository root in PowerShell:

```powershell
. ./scripts/activate.ps1
. ./scripts/rustflags.ps1
$env:VC_RS_ENABLE_NATIVE_TENSORRT = '0'
cargo test -p vc-gui --no-default-features onboarding::ui_tests -- --nocapture
```

Run all GUI unit tests with:

```powershell
cargo test -p vc-gui --no-default-features
```

`egui_kittest` is a development dependency matching the application's egui
minor version. Tests in `src/onboarding/ui_tests.rs` invoke the production
onboarding, basic controls, and conversion dialog widgets. They locate controls
by their visible labels and send pointer clicks, then check the resulting state.
The initial setup guard is exercised in English and Japanese.

The fixture supplies fixed engine/device state and creates an idle controller;
it does not call the normal startup, device enumeration, or settings autosave.
Persistence, device-test commands, engine starts and downloads are
injected effects in this fixture. Tests can click those tutorial controls and
inspect their recorded requests without using devices, networking or the user's
settings. Native file dialogs and inference remain outside this
fixture. No local models or microphone are required.

Normal-screen transport (including Stop failures and restart revision acknowledgement),
pending settings, passthrough pitch controls and per-role downloads are covered
by injected UI tests. The fixture shares production screen composition and checks
fixed transport and scrollable language/Setup controls at 520 × 480 in both languages.
Device selection tests cover input/output history order, duplicate removal,
disconnection/reconnection and returning to the system default. Reference downloads
use a nonexistent fixture cache path, independent of the user's installed models.
`render_normal_pages` with
`ui-snapshots` renders English/Japanese at 880 and 520 px widths to `target/normal-ui`.

This suite checks UI behavior, not pixel appearance or real audio. It does not
run the complete eframe lifecycle or validate Windows ML/TensorRT. Image
snapshots can be rendered with `--features ui-snapshots` and the ignored
`render_tutorial_pages` test; PNGs go to `target/onboarding-ui`. The test-only
Vulkan renderer differs from the application's `glow` renderer. See
[`docs/onboarding.md`](../../docs/onboarding.md) for device-test and rendering
commands and their limitations.
