# Progress

## 2026-09-29

- Switched from Loop GIF styling to the user's explicit global taskbar/focus diagnosis. The earlier acrylic probe is not running and remains only as an unexecuted test file.
- Confirmed branch and existing edits. Read Windows build, uptime, Explorer/DWM/Orbit processes, foreground registry values, and Microsoft's documented focus rules. No registry or shell values changed yet.
- Enumerated the live z-order, taskbar buttons, virtual desktop state, startup utilities, relevant hooks, and shell/error logs. No persistent topmost normal app or shell crash was found.
- Tested real taskbar clicks across Discord, File Explorer, Spotify, and Zeron, plus Alt+Tab. Clean checks passed. A later rapid sequence gave apparent failures while the user was moving/clicking the mouse, so it is not a valid reproduction.
- Added temporary read-only focus diagnostics under `.tools` to record click coordinates and foreground transitions. A controlled Chrome-to-Discord click activated Discord within 100 ms. Asked the user to try a failing taskbar click during passive tracing; the first 45-second trace contained no taskbar click.
- Did not change registry values, restart shell components, or run system repair because no cause has been isolated yet.
- A two-minute passive input/foreground trace recorded successful Discord and Chrome taskbar activations, including expected Chrome group thumbnail behavior. No clean failure occurred.
- Found Prism's shell hook injected into Explorer and read its source. It limits mouse consumption to the Start/Search rectangles, outside the live app-icon region. The installed Prism executable predates pre-existing source fixes for `HWND_TOPMOST` flashing on other apps and `HWND_BOTTOM` taskbar parking after fullscreen use. Ran `cargo test --locked taskbar::tests -j 2` in Prism: 4 passed. Preparing a release binary without changing the running app.
- `cargo build --locked --release --target x86_64-pc-windows-msvc -j 2` completed. With the user's “continue” after the deployment request, backed up the installed Prism exe, stopped the old process, installed the prepared build, and restarted with `--autostart`. Confirmed process and Explorer hook reload. The backup hash matches the old installed exe; the installed hash matches the new release artifact.
- Post-install taskbar clicks activated Spotify, File Explorer, Discord, and Windows Terminal within 100 ms. Alt+Tab switched Discord to Zeron and back. The intermittent original failure has not been observed since installation; waiting for user confirmation from normal use.
