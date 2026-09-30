# Match the Loop GIF

## Goal
Make Orbit's destination preview and radial indicator resemble `C:\Users\hi\Downloads\loop_demo.gif` on Windows while retaining click-through and target exclusions.

## Current state
Complete. Orbit uses an in-memory blurred destination snapshot for the plate and a neutral rounded-square radial. The release build is installed and running.

## Next action
Review the installed appearance during normal use and adjust only if a specific visual mismatch remains.

## Scope and constraints
- Preserve existing uncommitted work and do not commit.
- Do not launch the ignored `native_workflow` test.
- Do not change real application windows' materials; style Orbit's preview and indicator.
- Keep the foreground and z-order fix: layered popups must pass input to other processes.

## Completion criteria
- The preview reads as dark frosted glass with a subtle rounded edge and no saturated blue slab.
- The radial reads as a compact rounded square with a neutral body and light direction segment, matching the GIF at equivalent scale.
- The installed app shows the result on this Windows desktop; click-through, cancel, and exclusions still work.

## Phases
1. Inspect GIF, Loop source, Orbit paths, and Windows material options. Status: complete.
2. Implement the chosen material and geometry. Status: complete.
3. Run targeted checks, inspect the live desktop, and install. Status: complete.

## Decisions
- Use the GIF as the visual target. The pinned Loop source explains the material and stroke, but its current defaults may differ from the recorded GIF.
- Prefer a documented system material if it works with the existing popup contract. Use a controlled fallback if it does not.
- The DWM API accepted the transient backdrop attribute, but the captured popup remained flat gray. Use an in-memory blurred screen sample under the plate tint, with a normal translucent color-key fallback if screen capture fails.

## Blockers
Automated Windows foreground acquisition for a disposable test window became unreliable after a successful live capture. The final added radial edge was verified by a pixel test; the last visual desktop capture preceded that small adjustment.
