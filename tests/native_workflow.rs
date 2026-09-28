//! Runs on the interactive Windows desktop. Uses only a disposable target window and config.
use std::process::{Child, Command};
use std::thread::sleep;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HWND, POINT, RECT};
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
    let deadline = Instant::now() + Duration::from_secs(4);
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
    let _ = unsafe { windows::Win32::Graphics::Dwm::DwmGetWindowAttribute(window, windows::Win32::Graphics::Dwm::DWMWA_EXTENDED_FRAME_BOUNDS, (&mut rect as *mut RECT).cast(), std::mem::size_of::<RECT>() as u32) };
    (rect.left, rect.top, rect.right, rect.bottom)
}

#[test]
#[ignore = "takes foreground focus; run explicitly with --ignored --test-threads=1"]
fn radial_preview_cancel_commit_undo_and_settings() {
    unsafe { let _ = windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2); }
    assert!(
        unsafe { FindWindowW(w!("OrbitWindow"), w!("Orbit")) }.is_err(),
        "quit Orbit before the desktop test"
    );
    let config = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(config.path().join("Orbit")).unwrap();
    // Keep network and cosmetic inset out of the geometry assertions.
    std::fs::write(config.path().join("Orbit/settings.json"), br#"{"version":1,"preview_padding":0,"updates_enabled":false}"#).unwrap();
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
    key_event(VK_MENU, false);
    key_event(VK_MENU, true);
    assert!(
        unsafe { SetForegroundWindow(target).as_bool() },
        "Windows denied foreground focus to the disposable target"
    );
    wait_for(|| unsafe { GetForegroundWindow() } == target);
    let original = frame(target);
    let begin = || {
        wait_for(|| {
            [VK_CONTROL, VK_MENU, VK_SPACE, VK_ESCAPE]
                .iter()
                .all(|key| unsafe { GetAsyncKeyState(key.0 as i32) as u16 & 0x8000 == 0 })
        });
        key_event(VK_MENU, false);
        key_event(VK_MENU, true);
        assert!(unsafe { SetForegroundWindow(target).as_bool() });
        wait_for(|| unsafe { GetForegroundWindow() } == target);
        unsafe { SetCursorPos(400, 350) }.unwrap();
        for key in [VK_CONTROL, VK_MENU, VK_SPACE] {
            key_event(key, false);
        }
        wait_for(|| unsafe { IsWindowVisible(overlay).as_bool() });
        unsafe { SetCursorPos(490, 350) }.unwrap();
    };
    begin();
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
    assert_eq!(frame(preview), expected);
    key_event(VK_ESCAPE, false);
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
    wait_for(|| unsafe { IsWindowVisible(preview).as_bool() });
    for key in [VK_SPACE, VK_MENU, VK_CONTROL] {
        key_event(key, true);
    }
    wait_for(|| !unsafe { IsWindowVisible(overlay).as_bool() });
    wait_for(|| frame(target) != original);
    assert_eq!(frame(target), expected);
    for key in [VK_CONTROL, VK_MENU, VIRTUAL_KEY(u16::from(b'Z'))] {
        key_event(key, false);
    }
    for key in [VIRTUAL_KEY(u16::from(b'Z')), VK_MENU, VK_CONTROL] {
        key_event(key, true);
    }
    wait_for(|| frame(target) == original);
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
    unsafe { PostMessageW(Some(host), WM_CLOSE, Default::default(), Default::default()) }.unwrap();
    wait_for(|| guard.child.try_wait().unwrap().is_some());
}
