param([string]$Installer)
$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path "$PSScriptRoot\..").Path
if (-not $Installer) {
    $metadata = & cargo metadata --locked --no-deps --format-version 1 --manifest-path (Join-Path $projectRoot 'Cargo.toml') | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Cannot read package version' }
    $version = ($metadata.packages | Where-Object name -eq 'orbit').version
    $Installer = Join-Path $projectRoot "dist\OrbitSetup-$version-x64.exe"
}
$installRoot = Join-Path $projectRoot '.tools\installer-test'
$settingsRoot = Join-Path $env:LOCALAPPDATA 'Orbit'
$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\{BDF8B61C-26F2-4CEE-A76D-B24780DDC2E0}_is1'
$protocolKey = 'HKCU:\Software\Classes\orbit'
if ((Test-Path -LiteralPath $settingsRoot) -or (Test-Path -LiteralPath $uninstallKey) -or (Test-Path -LiteralPath $protocolKey) -or (Get-Process orbit -ErrorAction SilentlyContinue)) {
    throw 'Installer test needs no existing Orbit installation, settings, or running process. Existing data was left untouched.'
}
if ((Test-Path -LiteralPath $installRoot) -and (Get-ChildItem -Force -LiteralPath $installRoot)) {
    throw "Test install directory is not empty: $installRoot"
}
function Invoke-InstallerProcess([string]$File, [string[]]$Arguments) {
    # Inno's uninstaller launches a temporary child. -Wait waits for the process tree.
    $process = Start-Process -FilePath $File -ArgumentList $Arguments -WindowStyle Hidden -PassThru -Wait
    if ($process.ExitCode -ne 0) { throw "Installer returned $($process.ExitCode): $File" }
}
$setupArguments = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/NOICONS', "/DIR=`"$installRoot`"")
Invoke-InstallerProcess (Resolve-Path $Installer).Path $setupArguments
$executable = Join-Path $installRoot 'orbit.exe'
if (-not (Test-Path -LiteralPath $executable)) { throw 'Installed executable missing' }
if (-not (Test-Path -LiteralPath $uninstallKey)) { throw 'Per-user uninstall entry missing' }
$protocolCommand = (Get-Item -LiteralPath "$protocolKey\shell\open\command").GetValue('')
if ($protocolCommand -ne ('"' + $executable + '" "%1"')) { throw 'URI command registration is incorrect' }
New-Item -ItemType Directory -Path $settingsRoot | Out-Null
$settingsFile = Join-Path $settingsRoot 'settings.json'
$settingsContent = '{"version":1,"padding":17,"updates_enabled":false}'
[System.IO.File]::WriteAllText($settingsFile, $settingsContent)
$app = Start-Process -FilePath $executable -ArgumentList '--resident' -WindowStyle Hidden -PassThru
Start-Sleep -Milliseconds 600
if ($app.HasExited) { throw 'Installed Orbit exited during startup' }
Invoke-InstallerProcess $executable @('--quit')
if (-not $app.WaitForExit(10000)) { throw 'Installed Orbit did not quit' }
# Reinstall over the same version to exercise file replacement and settings preservation.
Invoke-InstallerProcess (Resolve-Path $Installer).Path $setupArguments
if ((Get-Content -Raw -LiteralPath $settingsFile) -ne $settingsContent) { throw 'Upgrade changed settings' }
# Exercise the updater's installer handoff against a running installed instance.
$beforeUpdate = Start-Process -FilePath $executable -ArgumentList '--resident' -WindowStyle Hidden -PassThru
Start-Sleep -Milliseconds 600
if ($beforeUpdate.HasExited) { throw 'Orbit did not start before the update handoff' }
$updateProcess = Start-Process -FilePath (Resolve-Path $Installer).Path -ArgumentList ($setupArguments + '/UPDATE=1') -WindowStyle Hidden -PassThru
# Do not wait for the entire process tree: a successful update intentionally starts a resident.
if (-not $updateProcess.WaitForExit(30000)) { throw 'Update installer did not finish' }
if ($updateProcess.ExitCode -ne 0) { throw "Update installer returned $($updateProcess.ExitCode)" }
if (-not $beforeUpdate.WaitForExit(10000)) { throw 'Update installer did not close the previous resident' }
$restartDeadline = [DateTime]::UtcNow.AddSeconds(10)
do {
    $restarted = @(Get-Process orbit -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $executable -and $_.Id -ne $beforeUpdate.Id })
    if ($restarted.Count -gt 0) { break }
    Start-Sleep -Milliseconds 100
} while ([DateTime]::UtcNow -lt $restartDeadline)
if ($restarted.Count -ne 1) { throw 'Update must restart exactly one installed resident' }
if ((Get-Content -Raw -LiteralPath $settingsFile) -ne $settingsContent) { throw 'Update handoff changed settings' }
$uninstaller = (Get-ItemProperty -LiteralPath $uninstallKey).UninstallString.Trim('"')
Invoke-InstallerProcess $uninstaller @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
if (-not $restarted[0].WaitForExit(10000)) { throw 'Uninstaller left its resident running' }
if (Test-Path -LiteralPath $executable) { throw 'Uninstall left the executable' }
if ((Get-Content -Raw -LiteralPath $settingsFile) -ne $settingsContent) { throw 'Default uninstall did not preserve settings' }
if (Test-Path -LiteralPath $uninstallKey) { throw 'Uninstall left its registry entry' }
if (Test-Path -LiteralPath $protocolKey) { throw 'Uninstall left its URI registration' }
Invoke-InstallerProcess (Resolve-Path $Installer).Path $setupArguments
$uninstaller = (Get-ItemProperty -LiteralPath $uninstallKey).UninstallString.Trim('"')
Invoke-InstallerProcess $uninstaller @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/REMOVESETTINGS=1')
if (Test-Path -LiteralPath $settingsRoot) { throw 'Explicit settings removal left settings' }
if (Test-Path -LiteralPath $executable) { throw 'Second uninstall left the executable' }
'status: passed'
'checks: install, launch, quit, reinstall, running_update_handoff, restart_after_update, running_uninstall, preserve_settings, remove_settings, uninstall_registry, uri_registration'
