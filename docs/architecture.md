# Windows architecture

Orbit uses one resident Win32 message loop. It owns the notification icon, global shortcuts, overlay windows, action history, stash state, and native settings window. Network work runs on background threads and returns results to the message loop.

Pure geometry, configuration validation, command parsing, and update verification live in the library. They can be tested without opening windows or changing desktop state.

## UI choice

The original plan proposed Windows Reactor and Windows Composition. Three approaches were considered:

| Approach | Deployment | Fit for this port |
| --- | --- | --- |
| Reactor and Windows App SDK | Requires bundling the matching runtime | A possible future richer UI, but the dependency generation did not match the existing windows-rs Win32 layer |
| Win32 controls and layered overlays | Uses APIs already available on supported Windows versions | Selected for the current implementation; native keyboard controls and a small per-user installer |
| WebView UI | Requires browser rendering/runtime integration | Outside the user's native-only requirement |

The current build uses native controls for settings and premultiplied pixels in layered windows for the radial menu. This is an implementation change from the Reactor/Composition proposal. The ring geometry, selector, sidebar, and settings panes follow the source design; exact visual parity remains subject to interactive review.

## Input and actions

The resident captures the target window when a trigger begins. Selection updates a preview. Commit uses the selected action; cancellation closes overlays without moving the target. Settings reload changes the active shortcut registrations and reports conflicts.

Commands use an allowlist of action names. URI commands support only window actions and opening settings. State-dependent commands go through the resident so history and stash operations share the same state as keyboard actions.

Window placement uses monitor work areas. A window property associates history with a live window, preventing recycled handles from inheriting another window's positions. Windows can destroy windows at any time, so each operation also validates its target.

## Updates and installation

The installer writes to the current user's Programs directory and registers notification-area launch, optional logon launch, the URI handler, and Windows uninstall metadata. It does not add a service or request elevation.

Update manifests are signed with Ed25519. The application enforces HTTPS, bounds network reads, rejects altered metadata, verifies the download hash, and checks the staged file again before launching it. A prompt precedes downloading and installation. Private signing material is never part of the application or source archive.

## Platform boundaries

Windows has no universal documented operation that enters another application's native fullscreen mode. Monitor-filling placement is an adaptation, and should not be described as native app fullscreen.

Numbered and relative virtual-desktop actions from macOS Spaces are excluded. macOS-only settings remain documented exceptions rather than inactive switches in the Windows UI.
