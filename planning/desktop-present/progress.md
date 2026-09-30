# Progress

## 2026-09-29

- Read the working tree, relevant paint paths and tests, and Loop's pinned SwiftUI source. Confirmed the requested fix is limited to popup presentation.
- Added the color key to both popup HWNDs, converted cached premultiplied pixels in `WM_PAINT`, and removed preview's `UpdateLayeredWindow` path. Preview is positioned and sized to its inset frame.
- Focused radial tests: 5 passed. Focused preview tests: 3 passed. `radial_pixels`: 10 passed. Ignored `layered_present` color-key probe: 1 passed. Pixel conversion test: 1 passed. Clippy with `-D warnings` and release build passed.
- `git diff --check` passed. The working tree already had substantial unrelated uncommitted edits; none were reverted or committed.
- The installed executable was replaced and started. A live capture showed the opaque plate and ring on the desktop; Escape hid both popups and preserved the test window after the Escape key was held until Orbit processed it. Direct `WM_NCHITTEST` returned `HTTRANSPARENT`.
- A large 944x1016 color-key DIB probe also passed on the desktop. The probe uses `SetDIBitsToDevice`, matching the production paint call.
- The user then stopped end-to-end testing. Cleared stale radial bitmap state before a new session, rebuilt the release executable, replaced the installed copy, and restarted Orbit. The installed SHA-256 is `FA0484788B2E3210A62EB28D59E5A7B8B8962278C54F61BB3AD01D6D436173A7` (PID 31380 at installation). No tests were run after the user's stop request. Hover under an opaque point of the plate remains to verify later.
