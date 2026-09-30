# Task plan

## Goal
Show Loop's placement preview: a rounded plate on a full-screen click-through panel, with a visible border, that does not steal hover.

## Current state
Phase 3 is complete. The installed app was restarted with the monitor-sized panel.

## Next action
User confirms the plate and border appear while Ctrl+Alt is held, and that hover still works.

## Scope and constraints
- Windows port stays windows-rs. Do not adopt GPUI.
- Do not commit or replace unrelated settings behavior.
- The running user app is `C:\Users\hi\AppData\Local\Programs\Orbit\orbit.exe`. Restart only that process after the build.
- Preview must ignore the mouse. Hover on windows underneath must keep working.

## Completion criteria
- Holding Ctrl+Alt and moving the pointer shows a rounded plate and a light border on the destination rectangle.
- The mouse still hovers controls under that plate.
- `cargo test --locked --bin orbit -- renders_interior_opacity gradient_interpolates renderer_keeps_raw_alpha` passes.

## Phases
1. Read Loop's preview panel and record the mechanism. Status: complete.
2. Draw the plate inside a monitor-sized click-through window. Status: complete.
3. Build, replace the installed exe, and restart Orbit. Status: complete.

## Decisions
- Follow Loop: one borderless panel the size of the target monitor, `ignoresMouseEvents`, plate drawn inside it. Do not keep resizing a small layered window to the target frame.
- HUD blur has no public per-window API here. Stand in with a dark translucent fill, then the accent wash, a 1 px light stroke, and the accent border.
- Existing alpha tests change to the new plate contract. See findings.md.

## Blockers
None identified.
