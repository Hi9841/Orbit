# Orbit motion, radial, and Settings polish

## Goal
Refine the radial to match the Loop GIF, animate destination preview transitions, restyle Settings to the same restrained theme, and reduce measured interaction cost.

## Current state
The measured baseline is about 28 ms for blur, render, and color-key conversion of a 944 x 1016 plate. Preview geometry now animates and converted pixels are cached. The radial cap geometry and Settings theme are ready for integrated visual inspection.

## Next action
Inspect the radial cap and Settings in the live app, then verify timer behavior, rendering, and build output.

## Scope and constraints
- Preserve existing uncommitted work. Do not commit.
- Do not run ignored `native_workflow` because it takes foreground focus.
- Keep Win32 popup click-through, game exclusions, and normal app z-order behavior.
- Keep the current Win32 architecture. GPUI is available, but a visual request does not justify replacing the popup stack.
- A bounded Settings-only worker handles `src/settings_window.rs`; the primary agent handles radial, preview motion, and render performance.

## Completion criteria
- Radial shape, stroke, selected arc, and light-background contrast look refined at 96 and 144 DPI.
- Changing directions moves and resizes the preview smoothly, can be interrupted, and finishes at the exact destination frame.
- Settings uses the app's dark neutral design with legible native controls and unchanged behavior.
- Targeted correctness checks, strict Clippy, live desktop inspection, and before/after performance measurements pass. Install the resulting build and preserve a rollback backup.

## Phases
1. Inspect the current motion and paint paths; capture baseline performance and visuals. Status: complete.
2. Implement radial and preview motion with measured paint improvements. Status: in_progress.
3. Integrate and verify the Settings redesign. Status: in_progress.
4. Build, install, and inspect the live result; record limits. Status: pending.

## Decisions
- Use the GIF as the shape/material reference while keeping Windows-specific input and focus contracts.
- The preview motion should explain a change in destination without delaying window movement after selection.

## Blockers
None.
