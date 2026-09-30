# Desktop preview and radial presentation

## Goal
Show Orbit's destination plate and radial ring on the Windows desktop using the working color-key layered-window paint path.

## Current state
Phases 1 and 2 complete. Phase 3 is blocked on the user's request to stop further desktop testing.

## Next action
When the user resumes desktop testing, check hover at a point inside both the destination plate and the underlying window. The installed release build is already running.

## Scope and constraints
- Keep existing user edits, preview opacity 255, opaque radial ring and cap, and `WM_NCHITTEST` click-through.
- Use the plate's destination frame for the preview HWND.
- Do not commit, run ignored `native_workflow`, or change unrelated settings and contracts.
- Inspect Loop's repository for the radial and preview styling reference.

## Completion criteria
- Both popups visibly paint on the Windows desktop and the preview matches the destination rect.
- Hover reaches the underlying window; Escape cancels without moving it.
- Requested tests, Clippy, release build, and installed executable replacement pass.

## Phases
1. Inspect Orbit state, measured failure, and Loop reference. Status: complete.
2. Implement color-key paint and remove the unused layered bitmap presenter. Status: complete.
3. Run requested checks and desktop verification; install and launch the new executable. Status: blocked. Plate and ring were observed on screen; hover under the plate awaits permission to resume desktop testing.

## Decisions
- The preview and radial remain native Win32 popups. Loop's SwiftUI source supplies visual proportions and colors; the measured local color-key probe supplies the Windows presentation path.

## Blockers
- The user asked to stop end-to-end testing for now. Do not run another desktop interaction check until they resume it.
