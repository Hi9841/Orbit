# Verification boundaries

## Current development build

On 2026-09-28, local `cargo test --locked --all-targets` passed 46 tests, with 3 desktop tests ignored. `cargo fmt --all` and `cargo clippy --locked --all-targets -- -D warnings` passed. Offscreen radial and preview bitmaps were rendered without opening a window. At that revision, preview opacity was a raw 0-255 bitmap alpha value.

On 2026-09-29, the local renderer changed to color-key Win32 paint with window-wide alpha, and fullscreen windows became excluded by default. The release executable built and started after these changes. A focused desktop probe reproduced a blocked click with `HTTRANSPARENT` alone and delivered it with `WS_EX_TRANSPARENT`; both installed Orbit popups have that style. The revised material and fullscreen exclusion have not received a full interactive workflow check.

Commit `2ea7203` was checked on a GitHub-hosted Windows Server 2025 runner:

- [Check Windows build 36455383244](https://github.com/Hi9841/Orbit/actions/runs/36455383244) passed formatting, locked tests, Clippy, and the release build.
- [Package run 36455396233](https://github.com/Hi9841/Orbit/actions/runs/36455396233) passed installer compilation, vendored source inspection, PE inspection, install, launch, quit, reinstall, running-app update handoff, preference preservation, uninstall with preferences kept and explicitly removed, native placement and history, shortcut cycling, radial preview, cancel, commit, animated placement, undo, stash and unstash, recovery after killing the resident, all eight settings pages, and saving preferences.
- The same native run held the middle button with an 80 ms trigger delay, selected a sector, released, placed the window, and undid that placement.
- Settings captures from that run show literal ampersands in "General & behavior" and "About & updates", the labels "Trigger modifier side" and "Restart cycles after another action", numeric edit borders, and footer buttons clear of the sidebar and group frames.

These checks do not establish physical Windows 10 or Windows 11 acceptance, mixed-DPI behavior, or screen-reader behavior. The hosted radial capture is a desktop screenshot around a layered overlay. The test asserts preview bounds. The ring and preview frame were also inspected from offscreen bitmaps. Do not claim exact Loop parity.

## Background checks

These commands compile or run pure tests without starting the app or changing desktop focus:

```powershell
cargo test --locked --lib
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --all --check
```

`cargo test --all-targets` is also safe while all tests that create or manipulate windows remain marked `#[ignore]`. Review new tests before running a broader suite.

## Interactive checks

The following checks open windows, move the cursor, or run installer dialogs. They require an explicitly approved, disposable interactive desktop:

```powershell
cargo test --test native_workflow -- --ignored --test-threads=1
cargo test --locked --test layered_present -- --ignored --nocapture --test-threads=1
cargo test --bin orbit -- --ignored --test-threads=1
./tools/Test-Installer.ps1
```

The installer test refuses to run when it detects existing Orbit settings, an installed copy, or a resident process. It uses a workspace install path and verifies per-user registration, launch, quit, reinstall, running-update handoff, exactly one restarted instance, running uninstall, settings preservation, explicit settings deletion, and uninstall registration removal.

The manually dispatched packaging workflow runs that installer test on a disposable GitHub-hosted Windows runner. This does not open anything on the developer's desktop. Hosted Windows Server results do not replace Windows 10/11 and mixed-DPI acceptance checks.

The native workflow test creates a disposable target, uses real key events, checks preview geometry, cancels, commits, undoes, stashes and restores through resident IPC, and tests hidden-window recovery after forced termination. It visits each settings pane and exits. It restores the pointer and previous foreground window when it finishes. Hosted runs capture the actual radial preview and settings panes; local runs never capture the desktop automatically.

The layered-present probe briefly shows two popups above its own test target, moves and restores the cursor, and sends a click. It checks desktop pixels, click delivery, and the installed popups' extended styles without activating or moving a normal window.

## Release acceptance

Verify these on both Windows 10 22H2 and Windows 11 x64 before claiming stable parity:

- Mixed-DPI monitors, negative monitor origins, taskbars on each edge, monitor disconnection.
- Normal, maximized, minimized, fixed-size, elevated, and closing target windows.
- Every radial sector, delay, keyboard shortcut, repeated cycle, and overlapping-key rejection.
- Stash, hover reveal, shutdown recovery, forced termination recovery, and exclusion changes.
- Settings keyboard navigation, high contrast, display scaling, screen-reader names, and error focus.
- Install, reinstall, real version upgrade, canceled update, uninstall with settings retained and removed.
- A signed update from the published feed, with tampered manifests and installers rejected.

No Mac runtime is available for executable comparison. Use the pinned Loop source and documented expectations, and label differences honestly in `migration/parity.md`.
