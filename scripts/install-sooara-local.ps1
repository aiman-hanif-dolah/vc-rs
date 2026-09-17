<#
.SYNOPSIS
    Install a local Windows ML build side by side without replacing old Sooara.
#>
[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [switch]$LaunchAtLogin,
    [switch]$StartAudioAtLogin
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$buildRoot = Join-Path $repoRoot 'target\release'
if (-not $env:LOCALAPPDATA) { throw 'LOCALAPPDATA is not set.' }
$files = @('vc-gui.exe', 'vc-rs.exe', 'Microsoft.WindowsAppRuntime.Bootstrap.dll')
foreach ($file in $files) {
    if (-not (Test-Path -LiteralPath (Join-Path $buildRoot $file) -PathType Leaf)) {
        throw "Missing $file. Build the Windows ML GUI and CLI and populate its bootstrapper first."
    }
}
$licenseRoot = Join-Path $buildRoot 'licenses'
$soundboardRoot = Join-Path $repoRoot 'resources\soundboard'
if (-not (Test-Path -LiteralPath (Join-Path $soundboardRoot 'KENNEY-LICENSE.txt'))) {
    throw 'Soundboard files and license material are missing.'
}
if (-not (Test-Path -LiteralPath $licenseRoot -PathType Container)) {
    throw 'Bootstrapper license material is missing.'
}
$version = Get-Date -Format 'yyyyMMdd-HHmmss-fff'
$destination = Join-Path $env:LOCALAPPDATA "Programs\SooaraNext\$version"
if (-not $PSCmdlet.ShouldProcess($destination, 'Install local Sooara preview and Start menu shortcut')) {
    return
}
New-Item -ItemType Directory -Path $destination | Out-Null
foreach ($file in $files) {
    $source = Join-Path $buildRoot $file
    $target = Join-Path $destination $file
    Copy-Item -LiteralPath $source -Destination $target
    if ((Get-FileHash -LiteralPath $source).Hash -ne (Get-FileHash -LiteralPath $target).Hash) {
        throw "Installed file hash mismatch: $file"
    }
}
Copy-Item -LiteralPath (Join-Path $repoRoot 'LICENSE') -Destination $destination
Copy-Item -LiteralPath $licenseRoot -Destination $destination -Recurse
Copy-Item -LiteralPath $soundboardRoot -Destination (Join-Path $destination 'soundboard') -Recurse
& (Join-Path $destination 'vc-rs.exe') doctor
if ($LASTEXITCODE -ne 0) { throw 'Installed runtime diagnostic failed; shortcut was not changed.' }
$startMenu = [Environment]::GetFolderPath('Programs')
if (-not $startMenu) { throw 'Cannot locate the per-user Start menu.' }
$startup = [Environment]::GetFolderPath('Startup')
if (-not $startup) { throw 'Cannot locate the per-user Startup folder.' }
$startupShortcut = Join-Path $startup 'Sooara Next.lnk'
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut((Join-Path $startMenu 'Sooara Next.lnk'))
$shortcut.TargetPath = Join-Path $destination 'vc-gui.exe'
$shortcut.WorkingDirectory = $destination
$shortcut.Description = 'Sooara local preview - noise reduction and voice conversion'
$shortcut.Save()
if ($LaunchAtLogin -or $StartAudioAtLogin -or (Test-Path -LiteralPath $startupShortcut)) {
    if ($PSCmdlet.ShouldProcess($startupShortcut, 'Configure Sooara login launch and saved audio startup preference')) {
        $loginShortcut = $shell.CreateShortcut($startupShortcut)
        $loginShortcut.TargetPath = $shortcut.TargetPath
        $loginShortcut.WorkingDirectory = $destination
        if ($PSBoundParameters.ContainsKey('StartAudioAtLogin')) {
            $loginShortcut.Arguments = $(if ($StartAudioAtLogin) { '--start' } else { '' })
        }
        $loginShortcut.Description = 'Open Sooara at login with the selected audio startup preference'
        $loginShortcut.Save()
        Write-Host "Sooara login launch configured. Arguments: $($loginShortcut.Arguments)"
    }
}
Write-Host "Installed local preview: $destination"
Write-Host 'Old Sooara, recordings, model files, and audio drivers were not changed.'
