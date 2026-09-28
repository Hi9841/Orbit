# Verification boundaries

## Current development build

On 2026-09-28, `cargo test --locked --all-targets` passed 32 tests, with three desktop tests explicitly ignored. Formatting and Clippy with warnings denied passed. The radial bitmap was inspected offscreen at 96 and 144 DPI. PE inspection confirmed an x64 Windows GUI executable without a Visual C++ redistributable import.

The current final installer has been compiled, but its interactive lifecycle has not been rerun locally. Earlier installer checks predate the expanded runtime. No Windows 10/11 clean-machine, screen-reader, mixed-DPI interaction, or published update acceptance result is claimed.

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
cargo test --bin orbit -- --ignored --test-threads=1
./tools/Test-Installer.ps1
```

The installer test refuses to run when it detects existing Orbit settings, an installed copy, or a resident process. It uses a workspace install path and verifies per-user registration, launch, quit, reinstall, settings preservation, explicit settings deletion, and uninstall registration removal.

The manually dispatched packaging workflow runs that installer test on a disposable GitHub-hosted Windows runner. This does not open anything on the developer's desktop. Hosted Windows Server results do not replace Windows 10/11 and mixed-DPI acceptance checks.

The native workflow test creates a disposable target, uses real key events, checks preview geometry, cancels, commits, undoes, opens settings, and exits. It restores the pointer and previous foreground window when it finishes.

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
