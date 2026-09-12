# Processing component terms — revision 2026-09-11

vc-rs is MIT licensed. Its license does not replace the licenses of downloaded
models, Windows ML, or GPU vendor components. Agreeing in the app covers the
applicable linked terms below; downloading support models is a separate action.

Windows ML distribution: Windows App SDK Foundation 2.3.9 bootstrapper, with
Windows App SDK Runtime 2.x (minimum 2.1) installed separately. The bootstrapper
terms are reproduced in WindowsAppSDK-Onboarding.txt and checked against the
SDK license by the Windows ML packaging script.

Windows ML Runtime license (2.1.74 baseline):
https://www.nuget.org/packages/Microsoft.Windows.AI.MachineLearning/2.1.74/License

Windows ML automatic selection can prepare NVIDIA TensorRT-RTX, Intel OpenVINO,
AMD MIGraphX/VitisAI, or Qualcomm QNN according to the device catalog. These
components are obtained and updated by Windows ML, not bundled in vc-rs.
Vendor-specific license links and compatibility requirements:
https://learn.microsoft.com/windows/ai/new-windows-ml/supported-execution-providers

Microsoft privacy statement:
https://privacy.microsoft.com/privacystatement

The native TensorRT package instead distributes TensorRT 11.2.1 and CUDA 13.3
runtime components under their respective licenses:
https://docs.nvidia.com/deeplearning/tensorrt/latest/reference/sla.html
https://docs.nvidia.com/cuda/eula/index.html

ContentVec and RMVPE are optional user-requested downloads (GPL-3.0), pinned to
the revision and checksums in the application's model acquisition module:
https://huggingface.co/wok000/weights_gpl

When changing SDK baselines, supported providers, or the applicable terms,
review this manifest and the authoritative licenses together. Its digest is
part of the GUI consent record. Never treat a tutorial skip as consent.
