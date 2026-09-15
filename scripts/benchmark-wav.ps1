<#
.SYNOPSIS
Repeat offline WAV timing measurements and retain environment and exact commands.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string]$InputWav,
    [Parameter(Mandatory)] [string]$Model,
    [Parameter(Mandatory)] [string]$Embedder,
    [Parameter(Mandatory)] [string]$F0Model,
    [Parameter(Mandatory)] [string]$OutputDirectory,
    [string]$Executable = 'target/release/vc-rs.exe',
    [string]$Provider = 'windowsml-openvino-gpu',
    [ValidateRange(1, 1000)] [int]$Repeat = 3,
    [ValidateRange(10, 10000)] [int]$ChunkMs = 500,
    [ValidateRange(0, 10000)] [int]$WarmupChunks = 3,
    [string]$CacheNote = 'Uncontrolled existing caches; no caches cleared by this script',
    [string[]]$ExtraArgs = @()
)
$ErrorActionPreference = 'Stop'
# Native diagnostics may fail for an unrelated backend. Retain their status;
# conversion failures below still abort the benchmark.
$PSNativeCommandUseErrorActionPreference = $false
foreach ($arg in $ExtraArgs) {
    if ($arg -match '^--(input|output|model|embedder|f0-model|provider|chunk-ms|performance-report|performance-warmup-chunks)(=|$)') {
        throw "Use the named benchmark parameter instead of overriding $arg in ExtraArgs"
    }
}
$exePath = (Resolve-Path -LiteralPath $Executable).Path
$inputPath = (Resolve-Path -LiteralPath $InputWav).Path
$modelPath = (Resolve-Path -LiteralPath $Model).Path
$embedderPath = (Resolve-Path -LiteralPath $Embedder).Path
$f0Path = (Resolve-Path -LiteralPath $F0Model).Path
if (Test-Path -LiteralPath $OutputDirectory) { throw 'OutputDirectory must be new' }
$destination = (New-Item -ItemType Directory -Path $OutputDirectory).FullName
function Read-EnvironmentField([scriptblock]$Action) {
    try { & $Action } catch { @{ unavailable = $_.Exception.Message } }
}
$manifest = [ordered]@{
    schema_version = 1
    started_utc = [DateTime]::UtcNow.ToString('o')
    cache_note = $CacheNote
    repetition_scope = 'Fresh process per run; disk/driver caches may be reused. No device placement proof.'
    os = Read-EnvironmentField { Get-CimInstance Win32_OperatingSystem | Select-Object Caption, Version, BuildNumber, OSArchitecture }
    cpu = Read-EnvironmentField { Get-CimInstance Win32_Processor | Select-Object Name, NumberOfCores, NumberOfLogicalProcessors }
    gpu = Read-EnvironmentField { Get-CimInstance Win32_VideoController | Select-Object Name, DriverVersion, PNPDeviceID }
    runtime_packages = Read-EnvironmentField { Get-AppxPackage '*WindowsAppRuntime*' | Select-Object Name, Version }
    git_revision = Read-EnvironmentField { git -C $PSScriptRoot rev-parse HEAD }
    git_status = Read-EnvironmentField { git -C $PSScriptRoot status --short }
    files = @($exePath, $inputPath, $modelPath, $embedderPath, $f0Path) | ForEach-Object { Get-FileHash -LiteralPath $_ -Algorithm SHA256 | Select-Object Path, Hash }
    # Models with external tensors need their sidecar files retained separately.
    model_hash_scope = 'ONNX files only; retain/hash external tensor sidecars separately if used'
    runs = @()
}
& $exePath doctor *> (Join-Path $destination 'doctor.log')
$manifest.doctor_exit_code = $LASTEXITCODE
for ($run = 1; $run -le $Repeat; $run++) {
    $wavPath = Join-Path $destination "run-$run.wav"
    $reportPath = Join-Path $destination "run-$run.json"
    $arguments = @('wav', '--input', $inputPath, '--output', $wavPath,
        '--model', $modelPath, '--embedder', $embedderPath, '--f0-model', $f0Path,
        '--provider', $Provider, '--chunk-ms', "$ChunkMs", '--performance-report', $reportPath,
        '--performance-warmup-chunks', "$WarmupChunks") + $ExtraArgs
    & $exePath @arguments *> (Join-Path $destination "run-$run.log")
    $runExitCode = $LASTEXITCODE
    # Query after loading: a catalog entry can lack version/path before it is prepared.
    & $exePath windowsml-eps list *> (Join-Path $destination "run-$run-catalog.log")
    $catalogExitCode = $LASTEXITCODE
    if ($runExitCode -eq 0) {
        $timings = Get-Content -LiteralPath $reportPath -Raw | ConvertFrom-Json
        $manifest["run_${run}_ep_libraries"] = @($timings.catalog_after_conversion | ForEach-Object {
            if ($_.library_path) {
                Read-EnvironmentField {
                    $library = Get-Item -LiteralPath $_.library_path
                    @{ path = $library.FullName; file_version = $library.VersionInfo.FileVersion;
                        product_version = $library.VersionInfo.ProductVersion;
                        sha256 = (Get-FileHash -LiteralPath $library.FullName).Hash }
                }
            }
        })
    }
    $manifest.runs += @{ index = $run; arguments = $arguments; exit_code = $runExitCode; catalog_exit_code = $catalogExitCode }
    $manifest | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath (Join-Path $destination 'environment.json') -Encoding utf8
    if ($runExitCode -ne 0) { throw "Conversion $run failed; see run-$run.log" }
    Write-Host "Completed run $run/$Repeat`: $reportPath"
}
