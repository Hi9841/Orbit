# Diagnose and repair global Windows focus

## Goal
Restore normal taskbar and Alt+Tab activation across unrelated Windows applications.

## Current state
The installed Prism build predates pre-existing source repairs for two direct z-order hazards: flashing unrelated app windows into `HWND_TOPMOST`, and parking Explorer's taskbar at `HWND_BOTTOM` after fullscreen use. The repaired Prism build is installed with a verified backup. Clean taskbar and Alt+Tab checks pass; the originally intermittent failure has not been captured before or after repair.

## Next action
Finish post-install checks while the user uses the desktop, then ask whether the intermittent symptom recurs. If it does, capture a clean failed click and use the saved old Prism executable for a controlled comparison.

## Scope and constraints
- Treat this as a system-wide issue; do not reset one application as a substitute for diagnosis.
- Prefer reversible Windows-standard repairs. Record original values before changing registry or shell configuration.
- Do not commit. Do not launch Orbit's ignored `native_workflow` test.
- Preserve unfinished Loop GIF prototype work in the workspace; it is separate from this system repair.

## Completion criteria
- Find a specific cause supported by measurements, make the smallest durable correction, and explain it.
- Click taskbar buttons for at least two unrelated apps and observe foreground HWND and visible z-order change correctly.
- Verify Alt+Tab still changes the foreground app.

## Phases
1. Capture current windows, foreground rules, Explorer/DWM, utilities, virtual desktops, and relevant configuration. Status: largely complete; waiting for a clean failed click.
2. Isolate the cause and apply a reversible repair. Status: candidate Prism root cause identified; repaired build installed with old executable backed up.
3. Verify taskbar and Alt+Tab behavior across apps; record final state. Status: clean checks passed, intermittent user symptom awaits confirmation.

## Decisions
- `ForegroundLockTimeout=200000` and `ForegroundFlashCount=7` are the saved values. The live timeout reads 2,147,483,647, which has also been observed on Windows 11 after sign-in; do not set either to zero as an unproven workaround.
- Do not treat a single failed automation sequence as a reproduction when the user was moving the mouse concurrently. A follow-up controlled click activated Discord within 100 ms.

## Blockers
The failure is intermittent and was not captured during a clean observed taskbar click before repair, so causal attribution to Prism remains provisional until normal use confirms it or a controlled old/new comparison reproduces it.
