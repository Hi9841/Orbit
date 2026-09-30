# Orbit

Native Windows window management in Rust using [windows-rs](https://github.com/microsoft/windows-rs). Orbit adapts [Loop](https://github.com/mrkai77/Loop)'s radial interaction and settings layout for Windows.

Orbit is GPL-3.0-only. It is a separate project with its own name and icon. See [NOTICE.md](NOTICE.md) for attribution.

## Install

Download the x64 installer from [Releases](https://github.com/Hi9841/Orbit/releases). Installation is per-user and does not request administrator access. The installer is unsigned, so Windows may show a publisher warning.

Target systems are Windows 10 22H2 and Windows 11, x64. No WebView or separate UI runtime is required.

## Use

1. Launch Orbit from the Start menu.
2. Focus the window you want to arrange.
3. Hold **Ctrl+Alt**, move the pointer toward a side or corner, then release. Change those modifiers in Settings. An optional extra key can be added there.
4. Press **Escape** before releasing to cancel. Use **Ctrl+Alt+Z** to undo.

Right-click Orbit's notification-area icon to open settings or quit. Preferences include the trigger, action shortcuts and cycles, custom frames, exclusions, snapping, radial appearance, and preview appearance.

## Uninstall

Remove Orbit from Windows Settings > Apps. The uninstaller asks whether to delete preferences and keeps them by default. It removes the logon entry and URI registration.

Silent uninstall keeps preferences. Add `/REMOVESETTINGS=1` to explicitly remove them.

## Scripts

```powershell
orbit left-half
orbit --list-actions
orbit --status
orbit --export-settings preferences.json
orbit --import-settings preferences.json
orbit --quit
```

Use the full path to `orbit.exe` if it is not on PATH. The installer registers `orbit://settings` and `orbit://action/left-half`. URI actions are limited to window actions and opening settings.

`--export-settings` requires a new file path. Import validates the full file before applying it. `--reset-settings` restores defaults. Exit codes are 0 for success, 1 for an operation failure, and 2 for invalid arguments.

## Build and check

Install a stable Rust MSVC toolchain and the Visual Studio C++ build tools, then run:

```powershell
cargo test --locked --lib
cargo check --locked --all-targets
cargo build --locked --release --bins
```

The app is `target/release/orbit.exe`. Build the installer with Inno Setup 6:

```powershell
./tools/Build-Release.ps1 -VendorDependencies
```

This creates an installer, corresponding source archive, dependency notices, and checksums in `dist`. The source archive includes dependency source when `-VendorDependencies` is supplied.

## Verification and limits

The [parity checklist](migration/parity.md) distinguishes implementation from desktop verification. Hosted Windows checks exercise real radial input, preview, undo, stash and crash recovery, settings, installation, update handoff, and uninstall. Windows 10/11 mixed-DPI and screen-reader acceptance still need testing. These checks do not prove exact visual or behavioral parity with Loop.

Desktop tests are opt-in because they open windows and take focus. Do not run ignored tests or `tools/Test-Installer.ps1` on a user's active desktop without their permission.

Numbered virtual-desktop moves and next/previous virtual-desktop switching are excluded because Windows does not provide a documented enumeration and switching equivalent. macOS-only features such as Stage Manager, Dock integration, haptics, and iCloud sync are not included. Fullscreen behavior and other adaptations are recorded in the parity checklist.

## Releases

See [the release guide](docs/releases.md) for building a source-matched release and signing its update manifest. The Windows executable and installer remain unsigned. Update metadata uses an independent Ed25519 signature and installer SHA-256 verification.
