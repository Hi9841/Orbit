# Orbit parity checklist

Reference: Loop `develop` at `df26d565e07c82e156b8f1c361bdcf428f32e3a4`.

This is a development port. Implementation does not establish desktop or visual parity. The source inventory contains 94 actions: 73 have command mappings, 18 are Spaces exceptions, and three represent no selection, configured custom frames, and configured cycles. Tests compare 29 source frame fractions at three monitor origins and sizes.

## Workflow coverage

| Workflow | Implementation | Remaining acceptance check |
| --- | --- | --- |
| Trigger and radial selection | Configurable modifier chord, delay, eight sectors, click/release commit, Escape cancellation, wheel selection | Every sector and input path on real applications |
| Radial appearance | Visibility, diameter, thickness, corner radius, custom solid accent, eight action assignments | Mixed-DPI appearance and source comparison; gradients, system accent, and animation remain unimplemented |
| Preview | Target-frame overlay, visibility, padding, opacity, corner radius | Cross-monitor alignment; source blur and separate border styling remain unimplemented |
| Shortcuts and cycles | Native binding editor, action sequences, timeout, Shift reversal, registration conflict rollback | Keyboard layout, reserved chords, rapid repeat, reload during use |
| Frames | Halves, quarters, thirds, fourths, centering, maximize, relative move/grow/shrink/scale, custom fractional frames | Fixed-size applications, minimum sizes, taskbars, display changes |
| Fullscreen | Monitor-filling placement without changing application window styles | Application-specific fullscreen behavior is not portable |
| Fill available space | Source-derived finite candidate search around visible neighbors | Real application overlap and z-order scenarios |
| Undo and initial frame | Bounded history with window identity markers and placement/style restoration | Closing/recycled windows, fullscreen, maximized and minimized state |
| Monitor movement and focus | Next/previous and directional monitor placement; directional and stack focus | Unequal DPI, negative origins, disconnected displays |
| Drag snapping | Title-bar move observation, edge preview and commit | Interaction with Windows Snap and canceled drags |
| Stash and hide | Edge stash, restore, hover recovery, persisted recovery journal | Forced termination, failed recovery, monitor removal, exclusion changes |
| Exclusions | Process-based exclusions applied by the runtime | Inaccessible and elevated processes |
| Scripting | Allowlisted CLI and URI actions, resident IPC, import/export/reset | Shell foreground targeting and URI invocation |
| Settings | Seven native sidebar panes, structured shortcut/custom-frame editors, DPI scaling, keyboard navigation | Screen reader, high contrast, small work areas; exact Loop visuals are not reproduced |
| Tray and logon | Settings/quit/undo menu and per-user startup registration | Explorer restart and clean-machine startup |
| Updates | HTTPS, signed manifest, bounded/hash-checked download, prompt, installer handoff | Published version upgrade and cancellation on Windows 10/11 |
| Installation | Per-user x64 installer, URI registration, uninstall with preserve/delete settings | Final-build lifecycle on clean Windows 10/11 |
| Spaces actions | Excluded | Windows has no documented equivalent for enumerating and switching numbered desktops |

## Settings coverage and differences

The generated inventory is [loop-settings.json](../tests/fixtures/loop-settings.json). Source settings are grouped below so omitted behavior is explicit.

| Source settings | Orbit mapping or status |
| --- | --- |
| `radialMenuVisibility`, `radialMenuCornerRadius`, `radialMenuThickness`, `radialMenuActions`, `customAccentColor` | Supported native radial settings |
| `accentColorMode`, `useGradient`, `gradientColor` | Pending; only a custom solid accent is available |
| `previewVisibility`, `previewPadding`, `previewCornerRadius`, `previewBackgroundAccentOpacity` | Supported; opacity uses Windows alpha units |
| `previewBorderThickness`, `previewUseWindowCornerRadius`, `previewBackgroundEnableBlur` | Pending |
| `launchAtLogin`, `windowSnapping`, `useScreenWithCursor`, `moveCursorWithWindow`, `resizeWindowUnderCursor`, `focusWindowOnResize` | Supported Windows settings |
| `enablePadding`, `padding` | One uniform inset; zero disables it. Source per-edge/configured padding is pending |
| `startHidden`, `hideMenuBarIcon` | Orbit starts without a main window and keeps its tray entry available |
| `animationConfiguration`, `animateStashedWindows`, `animateWindowResizes`, `ignoreLowPowerMode`, `previewStartingPosition` | Pending; placement and overlays update immediately |
| `restoreWindowFrameOnDrag`, `shiftFocusWhenStashed`, `cycleModeRestartEnabled` | Source options pending |
| `stashedWindowVisiblePadding`, `triggerKey`, `triggerDelay`, `cycleBackwardsOnShiftPressed`, `keybinds` | Supported; Windows uses a modifier plus virtual-key chord |
| `sideDependentTriggerKey`, `doubleClickToTrigger`, `middleClickTriggersLoop`, `enableTriggerDelayOnMiddleClick`, `triggerKeyTimeout` | Pending |
| `disableCursorInteraction`, `ignoreFullscreen`, `sizeIncrement`, `excludedApps` | Supported; exclusions use executable names |
| `hideOnNoSelection` | Pending; no selection cancels without changing the target |
| `enableRadialMenuCustomization` | Customization is always available |
| `useSystemWindowManagerWhenAvailable` | Orbit uses Windows placement APIs for its actions; native Snap layout delegation is not implemented |
| `updatesEnabled`, `automaticallyUpdate`, `includeDevelopmentVersions` | Stable-feed checks can be disabled; installation always requires confirmation; prerelease feed selection pending |
| `lockRadialMenuToCenter`, `snapThreshold` | Supported configuration fields |
| `paddingMinimumScreenSize` | Pending |
| `suppressMissionControlOnTopDrag`, `respectStageManager`, `stageStripSize`, `showDockIcon`, `hapticFeedback`, `ignoreNotch` | macOS-specific exceptions |
| `currentIcon`, `timesLooped`, `notificationWhenIconUnlocked` | Orbit uses one distinct icon; Loop icon-unlock rewards are omitted |
| `lastMigratorURL`, `patchesApplied` | Loop-specific migration state; Orbit uses a versioned configuration and explicit import |
| `stashManagerStashedWindows` | Windows recovery journal with process and live-window identity checks |
| `lastUsedAccentColor1`, `lastUsedAccentColor2` | No separate cache; current color is persisted |
| `showSettingsInspector` | Loop developer inspector omitted |

## Verification boundary

The Swift tests use `@testable import Loop` and cannot execute against this Windows binary. No Mac runtime is available for executable comparison. Use pinned source expectations without claiming an executable parity result.

Final interactive verification is intentionally outstanding because the user prohibited opening windows and interrupting their desktop. Tests that create windows remain ignored. Before a stable release, run the scenarios in [verification.md](../docs/verification.md) on disposable Windows 10 22H2 and Windows 11 x64 desktops. Record expected and actual frames, monitor, DPI, target app, and recovery outcomes.
