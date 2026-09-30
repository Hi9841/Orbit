# Native preview material and game targets

## Goal
Make the destination preview translucent and visually close to Loop's restrained HUD material, while preventing accidental moves of fullscreen games.

## Current state
Phases 1 and 2 complete. A focused click-through desktop probe passed after the user resumed testing; full material and game workflow checks remain open.

## Next action
When a foreground-changing workflow is authorized, inspect the new translucent preview and verify that Ctrl+Alt does nothing over a fullscreen game or the locally excluded Fortnite executable.

## Scope and constraints
- Preserve the working color-key GDI paint path and click-through behavior.
- Use documented Windows APIs and keep the Windows 10/11 target.
- Keep user settings and unrelated working-tree edits intact except for requested preview/game behavior.
- Do not commit or launch the ignored `native_workflow` test.
- A clarification is pending on whether windowed games should also be excluded.

## Completion criteria
- The preview has a neutral translucent fill with a restrained accent wash and visible edge; the radial remains legible and no longer reads as a solid white ring.
- Fullscreen or borderless fullscreen windows cannot be selected or moved by the resident trigger.
- The installed build uses the revised settings and starts successfully.
- A focused desktop click-through probe passed; full material and game workflow checks remain pending.

## Phases
1. Inspect Loop styling, Orbit rendering, target policy, and documented Windows APIs. Status: complete.
2. Implement the material and target rules, update relevant expectations, and build. Status: complete.
3. Install, inspect source diff, and check click-through. Status: partial. The installed build is running and the focused click probe passed; full visual and game checks remain open.

## Decisions
- Loop's default accent wash is 10% over a HUD material. Orbit currently renders a 100% blue wash and flattens all visible pixels to opaque. Use Win32 layered-window alpha for true translucency in the existing color-key presentation path.
- The existing `ignore_fullscreen` check runs during frame application and silently returns success. Move eligibility to target selection so the radial never activates on fullscreen windows.
- True blur is not available through `DwmEnableBlurBehindWindow` on Windows 8 and later; a documented Win32 translucent material is the baseline for Windows 10 and 11. See findings.
- Until the scope question is answered, exclude fullscreen and borderless fullscreen windows by default. The specific Fortnite executable seen in the live capture is also excluded in this user's settings, including when windowed. Windows has no reliable documented flag that classifies every windowed game; the existing per-process exclusions cover specific games.

## Blockers
- The ignored `native_workflow` test is explicitly excluded by user instruction.
- A generic policy for other windowed games awaits the user's answer.
