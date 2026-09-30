# Progress

## 2026-09-29

- Confirmed the branch and preserved existing edits. Extracted and inspected representative GIF frames and crops. Compared the recorded appearance with pinned Loop view code and Orbit render/settings paths.
- Identified the material mismatch and the saved border/radial shape mismatch. Next: test the documented Windows Acrylic path in a disposable popup before changing production rendering.
- Ran the disposable no-activate Acrylic probe and captured the result. The documented transient backdrop did not visibly blur on any tested popup style.
- Implemented a screen-snapshot blur under the preview tint and switched default radial geometry and accent to the GIF-like neutral square. Targeted renderer tests and all-target Clippy passed. Next: inspect the live appearance, then install the build and verify click-through and focus behavior without running `native_workflow`.
- Captured a live preview and radial over a normal test window in `.tools/loop-live.png`. The destination plate was dark, blurred, lightly outlined, and correctly sized to the right half. Escape kept the window at `(300,150,1060,670)`. Added a muted edge to the pale radial cap after noticing it blended into a white window.
- `cargo test --locked --bin orbit -- radial:: preview::` passed 10 tests; final `cargo test --locked --test radial_pixels` passed 12; ignored `layered_present` passed its visible and cross-process click-through checks; strict all-target Clippy and formatting passed; release build succeeded. The ignored `native_workflow` test was not run.
- Backed up the old installed binary and settings with suffix `pre-loop-ui-20260929-215536.bak`. Updated only saved appearance values: radial thickness 14, corner radius 20, neutral accent `#F4F4F0`, preview border 2, preview corner radius 8. Kept opacity 170 and game exclusion. Installed release SHA-256 `18FDDD527F9F1AD6A7F6E8681A49C70940F05A4CFCAC7EE6F826A3AB29BCCDE1`; Orbit PID 24288 started.
