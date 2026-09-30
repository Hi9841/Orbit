# Orbit parity checklist

Reference: Loop `develop` at `df26d565e07c82e156b8f1c361bdcf428f32e3a4`.

This is a development port. Implementation does not establish desktop or visual parity. The source inventory contains 94 actions: 73 have command mappings, 18 are Spaces exceptions, and three represent no selection, configured custom frames, and configured cycles. Tests compare 29 source frame fractions at three monitor origins and sizes.

## Workflow coverage

| Workflow | Implementation | Remaining acceptance check |
| --- | --- | --- |
| Trigger and radial selection | Configurable modifier chord, delay, eight sectors, click/release commit, Escape cancellation, wheel selection | Every sector and input path on real applications |
| Radial appearance | Visibility, diameter, thickness, corner radius, custom/system accent, gradient, eight action assignments | Mixed-DPI appearance and source comparison |
| Preview | Antialiased target-frame overlay, independent border/interior opacity, gradient, corner radius, configurable starting position | Cross-monitor alignment; macOS material blur is a Windows rendering exception |
| Shortcuts and cycles | Native binding editor, action sequences, timeout, Shift reversal, registration conflict rollback | Keyboard layout, reserved chords, rapid repeat, reload during use |
| Frames | Halves, quarters, thirds, fourths, centering, maximize, relative move/grow/shrink/scale, custom fractional frames | Fixed-size applications, minimum sizes, taskbars, display changes |
| Fullscreen | Monitor-filling placement without changing application window styles | Existing fullscreen and borderless fullscreen windows are excluded by default. Application-specific fullscreen behavior is not portable. |
| Fill available space | Source-derived finite candidate search around visible neighbors | Real application overlap and z-order scenarios |
| Undo and initial frame | Bounded history with window identity markers and placement/style restoration | Closing/recycled windows, fullscreen, maximized and minimized state |
| Monitor movement and focus | Next/previous and directional monitor placement; directional and stack focus | Unequal DPI, negative origins, disconnected displays |
| Drag snapping | Title-bar move observation, edge preview and commit | Interaction with Windows Snap and canceled drags |
| Stash and hide | Edge stash, restore, hover recovery, persisted recovery journal | Forced termination, failed recovery, monitor removal, exclusion changes |
| Exclusions | Process-based exclusions applied by the runtime | Inaccessible and elevated processes |
| Scripting | Allowlisted CLI and URI actions, resident IPC, import/export/reset | Shell foreground targeting and URI invocation |
| Settings | Eight native sidebar panes, structured shortcut/custom-frame editors, DPI scaling, keyboard navigation | Screen reader, high contrast, small work areas; exact Loop visuals are not reproduced |
| Tray and logon | Settings/quit/undo menu, optional icon visibility, Explorer restart registration, per-user startup | Clean-machine startup and Explorer restart interaction |
| Updates | HTTPS, signed manifest, bounded/hash-checked download, prompt, installer handoff | Published version upgrade and cancellation on Windows 10/11 |
| Installation | Per-user x64 installer, URI registration, uninstall with preserve/delete settings | Final-build lifecycle on clean Windows 10/11 |
| Spaces actions | Excluded | Windows has no documented equivalent for enumerating and switching numbered desktops |

## Settings coverage and differences

The generated inventory is [loop-settings.json](../tests/fixtures/loop-settings.json). Source settings are grouped below so omitted behavior is explicit.

| Source settings | Orbit mapping or status |
| --- | --- |
| `radialMenuVisibility`, `radialMenuCornerRadius`, `radialMenuThickness`, `radialMenuActions`, `customAccentColor` | Supported native radial settings |
| `accentColorMode`, `useGradient`, `gradientColor` | Custom/system accent and horizontal gradient |
| `previewVisibility`, `previewPadding`, `previewCornerRadius`, `previewBackgroundAccentOpacity` | Supported geometry and visibility. Orbit uses Loop's default 10% accent wash; its separate preview opacity setting controls Windows layered-window translucency. |
| `previewBorderThickness`, `previewUseWindowCornerRadius`, `previewBackgroundEnableBlur` | Independent border and standard Windows 8-DIP corner option. Other applications' actual corner radii cannot be queried. The Windows 10-compatible renderer uses a translucent HUD tint without macOS material blur. |
| `launchAtLogin`, `windowSnapping`, `useScreenWithCursor`, `moveCursorWithWindow`, `resizeWindowUnderCursor`, `focusWindowOnResize` | Supported Windows settings |
| `enablePadding`, `padding` | Uniform or per-edge inset; zero disables it |
| `startHidden`, `hideMenuBarIcon` | Orbit starts without a main window; tray can be hidden and settings remain accessible through `orbit --settings` |
| `animationConfiguration`, `animateStashedWindows`, `animateWindowResizes`, `ignoreLowPowerMode`, `previewStartingPosition` | Configurable duration with timer-based easing, window/stash animation, system motion and power preferences, preview origin |
| `restoreWindowFrameOnDrag`, `shiftFocusWhenStashed`, `cycleModeRestartEnabled` | Windows runtime options |
| `stashedWindowVisiblePadding`, `triggerKey`, `triggerDelay`, `cycleBackwardsOnShiftPressed`, `keybinds` | Supported; Windows uses a modifier plus virtual-key chord |
| `sideDependentTriggerKey`, `doubleClickToTrigger`, `middleClickTriggersLoop`, `enableTriggerDelayOnMiddleClick`, `triggerKeyTimeout` | Left/right modifier sides, double-tap chord, middle-button trigger with optional delay, trigger timeout |
| `disableCursorInteraction`, `ignoreFullscreen`, `sizeIncrement`, `excludedApps` | Supported; exclusions use executable names |
| `hideOnNoSelection` | Hide the radial indicator when no action is selected |
| `enableRadialMenuCustomization` | Customization is always available |
| `useSystemWindowManagerWhenAvailable` | Orbit uses Windows placement APIs for its actions; native Snap layout delegation is not implemented |
| `updatesEnabled`, `automaticallyUpdate`, `includeDevelopmentVersions` | Checks can be disabled; stable/development feeds; installation always requires confirmation |
| `lockRadialMenuToCenter`, `snapThreshold` | Supported configuration fields |
| `paddingMinimumScreenSize` | Physical display diagonal threshold; padding is skipped when the display measurement is unavailable |
| `suppressMissionControlOnTopDrag`, `respectStageManager`, `stageStripSize`, `showDockIcon`, `hapticFeedback`, `ignoreNotch` | macOS-specific exceptions |
| `currentIcon`, `timesLooped`, `notificationWhenIconUnlocked` | Orbit uses one distinct icon; Loop icon-unlock rewards are omitted |
| `lastMigratorURL`, `patchesApplied` | Loop-specific migration state; Orbit uses a versioned configuration and explicit import |
| `stashManagerStashedWindows` | Windows recovery journal with process and live-window identity checks |
| `lastUsedAccentColor1`, `lastUsedAccentColor2` | No separate cache; current color is persisted |
| `showSettingsInspector` | Loop developer inspector omitted |

## Verification boundary

The Swift tests use `@testable import Loop` and cannot execute against this Windows binary. No Mac runtime is available for executable comparison. Use pinned source expectations without claiming an executable parity result.

Interactive tests run on disposable GitHub-hosted Windows runners. They remain ignored by default to avoid opening windows on a user's active desktop. Package run [36455396233](https://github.com/Hi9841/Orbit/actions/runs/36455396233) on Windows Server 2025 passed radial preview, cancel, commit, animated placement, undo, shortcut cycling, delayed middle-button trigger, stash and restore, forced-termination recovery, all eight settings pages, and the installer, update handoff, and uninstall lifecycle. Check run [36455383244](https://github.com/Hi9841/Orbit/actions/runs/36455383244) passed formatting, locked tests, Clippy, and the release build. These results do not establish Windows 10/11, mixed-DPI, or screen-reader acceptance, and they are not exact Loop parity.

Intentional Windows adaptations include: macOS Spaces and Stage Manager actions are unavailable because Windows has no documented numbered-desktop enumeration and switching API; the preview corner option uses the standard 8-DIP Windows corner rather than another application's measured radius; macOS material blur, Dock, haptics, and the notch are omitted. See [verification.md](../docs/verification.md).
