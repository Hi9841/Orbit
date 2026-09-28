//! Runs on the interactive Windows desktop. Uses only a disposable target window and config.
use std::process::{Child, Command};
use std::thread::sleep;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;

struct DesktopTest {
    child: Child,
    target: HWND,
    cursor: POINT,
    foreground: HWND,
}
impl Drop for DesktopTest {
    fn drop(&mut self) {
        for key in [VK_CONTROL, VK_MENU, VK_SPACE, VK_ESCAPE] {
            key_event(key, true);
        }
        if let Ok(host) = unsafe { FindWindowW(w!("OrbitWindow"), w!("Orbit")) } {
            let _ = unsafe {
                PostMessageW(Some(host), WM_CLOSE, Default::default(), Default::default())
            };
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        unsafe {
            let _ = DestroyWindow(self.target);
            let _ = SetCursorPos(self.cursor.x, self.cursor.y);
            let _ = SetForegroundWindow(self.foreground);
        }
    }
}

fn key_event(key: VIRTUAL_KEY, up: bool) {
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                ..Default::default()
            },
        },
    };
    assert_eq!(
        unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) },
        1
    );
}

#[track_caller]
fn wait_for(mut condition: impl FnMut() -> bool) {
    wait_for_timeout(Duration::from_secs(4), &mut condition);
}

#[track_caller]
fn wait_for_timeout(timeout: Duration, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + timeout;
    loop {
        let mut message = MSG::default();
        while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() } {
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        if condition() {
            return;
        }
        assert!(Instant::now() < deadline, "native workflow timed out");
        sleep(Duration::from_millis(10));
    }
}

fn frame(window: HWND) -> (i32, i32, i32, i32) {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(window, &mut rect) }.unwrap();
    let _ = unsafe {
        windows::Win32::Graphics::Dwm::DwmGetWindowAttribute(
            window,
            windows::Win32::Graphics::Dwm::DWMWA_EXTENDED_FRAME_BOUNDS,
            (&mut rect as *mut RECT).cast(),
            std::mem::size_of::<RECT>() as u32,
        )
    };
    (rect.left, rect.top, rect.right, rect.bottom)
}

fn capture(name: &str, window: Option<HWND>) {
    let Some(directory) = std::env::var_os("ORBIT_DESKTOP_CAPTURE") else {
        return;
    };
    use std::os::windows::process::CommandExt;
    std::fs::create_dir_all(&directory).unwrap();
    let output = std::path::PathBuf::from(directory).join(format!("{name}.png"));
    let mut command = Command::new("powershell.exe");
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-File",
            "tools/Capture-TestDesktop.ps1",
            "-Output",
        ])
        .arg(output)
        .creation_flags(0x08000000);
    if let Some(window) = window {
        let (left, top, right, bottom) = frame(window);
        command.args([
            "-Left",
            &left.to_string(),
            "-Top",
            &top.to_string(),
            "-Width",
            &(right - left).to_string(),
            "-Height",
            &(bottom - top).to_string(),
        ]);
    }
    let mut process = command.spawn().unwrap();
    // Cold PowerShell/.NET startup on hosted runners can take over four seconds.
    wait_for_timeout(Duration::from_secs(15), || {
        process.try_wait().unwrap().is_some()
    });
    assert!(process.wait().unwrap().success());
}

fn dispatch(host: HWND, target: HWND, action: orbit::geometry::Action) {
    request(
        host,
        serde_json::json!({
            "version": 1, "target": target.0 as isize, "action": action
        }),
    );
}

fn request(host: HWND, value: serde_json::Value) {
    use windows::Win32::System::DataExchange::COPYDATASTRUCT;
    let mut payload = serde_json::to_vec(&value).unwrap();
    let packet = COPYDATASTRUCT {
        dwData: 0x4f52_4254,
        cbData: payload.len() as u32,
        lpData: payload.as_mut_ptr().cast(),
    };
    let mut result = 0usize;
    // This thread owns the disposable target. Allow incoming sent messages while
    // the resident changes its placement, just as a normal app's event loop does.
    assert_ne!(
        unsafe {
            SendMessageTimeoutW(
                host,
                WM_COPYDATA,
                WPARAM(0),
                LPARAM((&packet as *const COPYDATASTRUCT) as isize),
                SMTO_ABORTIFHUNG,
                5000,
                Some(&mut result),
            )
        }
        .0,
        0,
        "resident IPC timed out"
    );
    assert_eq!(result as isize, 1, "resident rejected {value}");
}

fn middle_button(up: bool) {
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dwFlags: if up {
                    MOUSEEVENTF_MIDDLEUP
                } else {
                    MOUSEEVENTF_MIDDLEDOWN
                },
                ..Default::default()
            },
        },
    };
    assert_eq!(
        unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) },
        1
    );
}

#[test]
#[ignore = "takes foreground focus; run explicitly with --ignored --test-threads=1"]
fn radial_preview_cancel_commit_undo_and_settings() {
    // A desktop API can block before the polling deadline gets control again.
    // Keep the hosted check bounded and preserve its last logged step.
    std::thread::spawn(|| {
        sleep(Duration::from_secs(120));
        eprintln!("native workflow exceeded its 120 second deadline");
        std::process::exit(1);
    });
    eprintln!("native workflow: create disposable target");
    unsafe {
        let _ = windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(
            windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        );
    }
    assert!(
        unsafe { FindWindowW(w!("OrbitWindow"), w!("Orbit")) }.is_err(),
        "quit Orbit before the desktop test"
    );
    let config = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(config.path().join("Orbit")).unwrap();
    // Keep network and cosmetic inset out of the geometry assertions.
    std::fs::write(
        config.path().join("Orbit/settings.json"),
        br#"{"version":1,"preview_padding":0,"updates_enabled":false,"animate_window_resizes":true,"animate_stashed_windows":true,"animation_duration_ms":120,"shortcuts":[{"hotkey":{"key":90},"actions":["undo"]},{"hotkey":{"key":88},"actions":["left_half","right_half"]}]}"#,
    )
    .unwrap();
    let foreground = unsafe { GetForegroundWindow() };
    let mut cursor = POINT::default();
    unsafe { GetCursorPos(&mut cursor) }.unwrap();
    let target = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("Orbit disposable workflow target"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            160,
            160,
            480,
            320,
            None,
            None,
            None,
            None,
        )
    }
    .unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_orbit"))
        .arg("--resident")
        .env("LOCALAPPDATA", config.path())
        .spawn()
        .unwrap();
    let mut guard = DesktopTest {
        child,
        target,
        cursor,
        foreground,
    };
    let mut host = HWND::default();
    eprintln!("native workflow: wait for resident");
    wait_for(|| {
        host = unsafe { FindWindowW(w!("OrbitWindow"), w!("Orbit")) }.unwrap_or_default();
        !host.0.is_null()
    });
    let mut overlay = HWND::default();
    wait_for(|| {
        overlay =
            unsafe { FindWindowW(w!("OrbitWindow"), w!("Orbit radial menu")) }.unwrap_or_default();
        !overlay.0.is_null()
    });
    // Wait for startup to finish registering both global bindings.
    sleep(Duration::from_millis(150));
    eprintln!("native workflow: focus target");
    // Establish last-input ownership without activating an Alt system-menu loop.
    key_event(VK_F24, false);
    key_event(VK_F24, true);
    eprintln!("native workflow: request foreground target");
    assert!(
        unsafe { SetForegroundWindow(target).as_bool() },
        "Windows denied foreground focus to the disposable target"
    );
    wait_for(|| unsafe { GetForegroundWindow() } == target);
    eprintln!("native workflow: target focused");
    let original = frame(target);
    eprintln!("native workflow: cycle shortcut on a previously untracked window");
    {
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        assert!(unsafe {
            GetMonitorInfoW(
                MonitorFromWindow(target, MONITOR_DEFAULTTONEAREST),
                &mut info,
            )
            .as_bool()
        });
        let area = info.rcWork;
        let middle = area.left + (area.right - area.left) / 2;
        for expected in [
            (area.left, area.top, middle, area.bottom),
            (middle, area.top, area.right, area.bottom),
        ] {
            for key in [VK_CONTROL, VK_MENU, VIRTUAL_KEY(u16::from(b'X'))] {
                key_event(key, false);
            }
            for key in [VIRTUAL_KEY(u16::from(b'X')), VK_MENU, VK_CONTROL] {
                key_event(key, true);
            }
            wait_for(|| frame(target) == expected);
        }
        dispatch(host, target, orbit::geometry::Action::Undo);
        dispatch(host, target, orbit::geometry::Action::Undo);
        wait_for(|| frame(target) == original);
    }
    let begin = || {
        eprintln!("native workflow: release keys and begin radial");
        wait_for(|| {
            [VK_CONTROL, VK_MENU, VK_SPACE, VK_ESCAPE]
                .iter()
                .all(|key| unsafe { GetAsyncKeyState(key.0 as i32) as u16 & 0x8000 == 0 })
        });
        key_event(VK_F24, false);
        key_event(VK_F24, true);
        assert!(unsafe { SetForegroundWindow(target).as_bool() });
        wait_for(|| unsafe { GetForegroundWindow() } == target);
        unsafe { SetCursorPos(400, 350) }.unwrap();
        for key in [VK_CONTROL, VK_MENU, VK_SPACE] {
            key_event(key, false);
        }
        wait_for(|| unsafe { IsWindowVisible(overlay).as_bool() });
        eprintln!("native workflow: radial visible");
        unsafe { SetCursorPos(490, 350) }.unwrap();
    };
    begin();
    eprintln!("native workflow: inspect preview");
    let preview = unsafe { FindWindowW(w!("OrbitWindow"), w!("Orbit preview")) }.unwrap();
    wait_for(|| unsafe { IsWindowVisible(preview).as_bool() });
    assert_eq!(frame(target), original, "preview must not move the target");
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    assert!(unsafe {
        GetMonitorInfoW(
            MonitorFromWindow(target, MONITOR_DEFAULTTONEAREST),
            &mut info,
        )
        .as_bool()
    });
    let area = info.rcWork;
    let expected = (
        area.left + (area.right - area.left) / 2,
        area.top,
        area.right,
        area.bottom,
    );
    wait_for(|| frame(preview) == expected);
    capture("radial-preview", None);
    key_event(VK_ESCAPE, false);
    eprintln!("native workflow: cancel radial");
    wait_for(|| !unsafe { IsWindowVisible(overlay).as_bool() });
    assert!(!unsafe { IsWindowVisible(preview).as_bool() });
    for key in [VK_ESCAPE, VK_SPACE, VK_MENU, VK_CONTROL] {
        key_event(key, true);
    }
    assert_eq!(
        frame(target),
        original,
        "Escape must preserve the original frame"
    );
    begin();
    eprintln!("native workflow: commit radial");
    wait_for(|| unsafe { IsWindowVisible(preview).as_bool() });
    for key in [VK_SPACE, VK_MENU, VK_CONTROL] {
        key_event(key, true);
    }
    wait_for(|| !unsafe { IsWindowVisible(overlay).as_bool() });
    wait_for(|| frame(target) == expected);
    for key in [VK_CONTROL, VK_MENU, VIRTUAL_KEY(u16::from(b'Z'))] {
        key_event(key, false);
    }
    for key in [VIRTUAL_KEY(u16::from(b'Z')), VK_MENU, VK_CONTROL] {
        key_event(key, true);
    }
    wait_for(|| frame(target) == original);
    eprintln!("native workflow: stash and restore through resident IPC");
    dispatch(host, target, orbit::geometry::Action::StashLeft);
    wait_for(|| frame(target).0 < area.left);
    dispatch(host, target, orbit::geometry::Action::Unstash);
    wait_for(|| frame(target) == original);
    eprintln!("native workflow: delayed middle-button trigger");
    let settings_path = config.path().join("Orbit/settings.json");
    let mut preferences: orbit::settings::Settings =
        serde_json::from_slice(&std::fs::read(&settings_path).unwrap()).unwrap();
    preferences.middle_click_triggers = true;
    preferences.middle_click_uses_delay = true;
    preferences.trigger_delay_ms = 80;
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&preferences).unwrap(),
    )
    .unwrap();
    request(host, serde_json::json!({"version":1,"reload":true}));
    unsafe { SetCursorPos(400, 350) }.unwrap();
    middle_button(false);
    wait_for(|| unsafe { IsWindowVisible(overlay).as_bool() });
    unsafe { SetCursorPos(490, 350) }.unwrap();
    wait_for(|| unsafe { IsWindowVisible(preview).as_bool() });
    middle_button(true);
    wait_for(|| !unsafe { IsWindowVisible(overlay).as_bool() });
    wait_for(|| frame(target) == expected);
    dispatch(host, target, orbit::geometry::Action::Undo);
    wait_for(|| frame(target) == original);
    preferences.middle_click_triggers = false;
    preferences.middle_click_uses_delay = false;
    preferences.trigger_delay_ms = 0;
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&preferences).unwrap(),
    )
    .unwrap();
    request(host, serde_json::json!({"version":1,"reload":true}));
    eprintln!("native workflow: recover hidden target after forced termination");
    dispatch(host, target, orbit::geometry::Action::Hide);
    wait_for(|| !unsafe { IsWindowVisible(target).as_bool() });
    guard.child.kill().unwrap();
    guard.child.wait().unwrap();
    guard.child = Command::new(env!("CARGO_BIN_EXE_orbit"))
        .arg("--resident")
        .env("LOCALAPPDATA", config.path())
        .spawn()
        .unwrap();
    wait_for(|| {
        host = unsafe { FindWindowW(w!("OrbitWindow"), w!("Orbit")) }.unwrap_or_default();
        !host.0.is_null()
            && unsafe { IsWindowVisible(target).as_bool() }
            && frame(target) == original
    });
    eprintln!("native workflow: open settings");
    // Exercise the command through the executable, then inspect the real settings window.
    assert!(
        Command::new(env!("CARGO_BIN_EXE_orbit"))
            .arg("--settings")
            .status()
            .unwrap()
            .success()
    );
    wait_for(|| {
        unsafe { FindWindowW(w!("OrbitSettings"), None) }
            .is_ok_and(|h| unsafe { IsWindowVisible(h).as_bool() })
    });
    let settings = unsafe { FindWindowW(w!("OrbitSettings"), None) }.unwrap();
    let sidebar = unsafe { GetDlgItem(Some(settings), 10) }.unwrap();
    let count = unsafe { SendMessageW(sidebar, LB_GETCOUNT, None, None) }.0;
    assert_eq!(count, 8, "settings pages are missing");
    for index in 0..count {
        unsafe {
            SendMessageW(sidebar, LB_SETCURSEL, Some(WPARAM(index as usize)), None);
            SendMessageW(
                settings,
                WM_COMMAND,
                Some(WPARAM(10 | ((LBN_SELCHANGE as usize) << 16))),
                Some(LPARAM(sidebar.0 as isize)),
            );
        }
        // Let normal paint messages settle before capturing the real controls.
        let painted = Instant::now() + Duration::from_millis(100);
        wait_for(|| Instant::now() >= painted);
        eprintln!("native workflow: settings page {index}");
        capture(&format!("settings-{index}"), Some(settings));
    }
    eprintln!("native workflow: save settings and confirm persistence");
    let before_save = std::fs::read(&settings_path).unwrap();
    unsafe {
        SendMessageW(settings, WM_COMMAND, Some(WPARAM(14)), None);
    }
    wait_for(|| std::fs::read(&settings_path).is_ok_and(|bytes| bytes != before_save));
    let saved: orbit::settings::Settings =
        serde_json::from_slice(&std::fs::read(config.path().join("Orbit/settings.json")).unwrap())
            .unwrap();
    saved.validate().unwrap();
    assert!(saved.animate_window_resizes && saved.animate_stashed_windows);
    assert!(!saved.updates_enabled);
    assert_eq!(saved.preview_padding, 0);
    assert_eq!(
        saved.preview_opacity,
        orbit::settings::Settings::default().preview_opacity
    );
    unsafe { PostMessageW(Some(host), WM_CLOSE, Default::default(), Default::default()) }.unwrap();
    eprintln!("native workflow: quit resident");
    wait_for(|| guard.child.try_wait().unwrap().is_some());
}
