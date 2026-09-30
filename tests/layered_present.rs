//! Confirms which present path actually lands on the desktop. Ignored by default.
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::thread::JoinHandle;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Dwm::DwmFlush;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BeginPaint, DIB_RGB_COLORS, EndPaint, GetDC, GetPixel,
    PAINTSTRUCT, ReleaseDC, SetDIBitsToDevice,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT, SendInput,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, FindWindowW, GWL_EXSTYLE,
    GetCursorPos, GetMessageW, GetWindowLongPtrW, HTTRANSPARENT, LWA_COLORKEY, MA_NOACTIVATE, MSG,
    PM_REMOVE, PeekMessageW, PostMessageW, PostQuitMessage, RegisterClassW, SW_HIDE,
    SW_SHOWNOACTIVATE, SetCursorPos, SetLayeredWindowAttributes, ShowWindow, TranslateMessage,
    WINDOW_EX_STYLE, WM_CLOSE, WM_DESTROY, WM_LBUTTONDOWN, WM_MOUSEACTIVATE, WM_NCHITTEST,
    WM_PAINT, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP, WindowFromPoint,
};
use windows::core::w;

const WIDTH: i32 = 944;
const HEIGHT: i32 = 1016;
const CLICK_POINT: POINT = POINT { x: 1100, y: 500 };
static TARGET_CLICKS: AtomicU32 = AtomicU32::new(0);

struct TargetGuard {
    window: HWND,
    worker: Option<JoinHandle<()>>,
    cursor: POINT,
}

impl Drop for TargetGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = PostMessageW(Some(self.window), WM_CLOSE, WPARAM(0), LPARAM(0));
            let _ = SetCursorPos(self.cursor.x, self.cursor.y);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[test]
#[ignore = "shows a layered window on the interactive desktop"]
fn color_key_window_is_visible_and_click_through() {
    let class = WNDCLASSW {
        lpfnWndProc: Some(probe_wnd_proc),
        lpszClassName: w!("OrbitLayerProbe"),
        ..Default::default()
    };
    assert_ne!(unsafe { RegisterClassW(&class) }, 0);
    let target = start_target();
    let (without_style_pixel, without_style_hit, without_style_clicks) = probe(WINDOW_EX_STYLE(0));
    let (click_through_pixel, click_through_hit, click_through_clicks) = probe(WS_EX_TRANSPARENT);
    println!(
        "without WS_EX_TRANSPARENT: hit_probe={without_style_hit}, target_clicks={without_style_clicks}; with style: hit_probe={click_through_hit}, target_clicks={click_through_clicks}"
    );
    drop(target);
    assert_eq!(without_style_pixel, 0x00ff00ff);
    assert_eq!(click_through_pixel, 0x00ff00ff);
    assert!(
        !click_through_hit,
        "layered popup intercepted the underlying window"
    );
    assert_eq!(click_through_clicks, 1, "click did not reach the target");
    for title in [w!("Orbit preview"), w!("Orbit radial menu")] {
        let window = unsafe { FindWindowW(w!("OrbitWindow"), title) }.expect("Orbit popup exists");
        let style = unsafe { GetWindowLongPtrW(window, GWL_EXSTYLE) as u32 };
        assert_ne!(style & WS_EX_LAYERED.0, 0);
        assert_ne!(style & WS_EX_TRANSPARENT.0, 0);
    }
}

fn start_target() -> TargetGuard {
    let mut cursor = POINT::default();
    unsafe { GetCursorPos(&mut cursor) }.expect("save cursor");
    let (sender, receiver) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let class = WNDCLASSW {
            lpfnWndProc: Some(target_wnd_proc),
            lpszClassName: w!("OrbitHitTarget"),
            ..Default::default()
        };
        assert_ne!(unsafe { RegisterClassW(&class) }, 0);
        let target = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                w!("OrbitHitTarget"),
                w!("Orbit hit target"),
                WS_POPUP,
                968,
                8,
                WIDTH,
                HEIGHT,
                None,
                None,
                None,
                None,
            )
        }
        .expect("create hit target");
        unsafe {
            let _ = ShowWindow(target, SW_SHOWNOACTIVATE);
        }
        sender.send(target.0 as usize).expect("send target handle");
        let mut message = MSG::default();
        while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    });
    let window = HWND(receiver.recv().expect("receive target handle") as *mut _);
    TargetGuard {
        window,
        worker: Some(worker),
        cursor,
    }
}

fn probe(extra_style: WINDOW_EX_STYLE) -> (u32, bool, u32) {
    let window = unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | extra_style,
            w!("OrbitLayerProbe"),
            w!("Orbit layer probe"),
            WS_POPUP,
            968,
            8,
            WIDTH,
            HEIGHT,
            None,
            None,
            None,
            None,
        )
    }
    .expect("create");
    unsafe {
        SetLayeredWindowAttributes(window, COLORREF(0x0000ff00), 255, LWA_COLORKEY)
            .expect("color key");
        let _ = ShowWindow(window, SW_SHOWNOACTIVATE);
    }
    pump();
    let _ = unsafe { DwmFlush() };
    std::thread::sleep(std::time::Duration::from_millis(40));
    let dc = unsafe { GetDC(None) };
    let pixel = unsafe { GetPixel(dc, 1100, 500) };
    let hit = unsafe { WindowFromPoint(CLICK_POINT) };
    TARGET_CLICKS.store(0, Ordering::Release);
    unsafe { SetCursorPos(CLICK_POINT.x, CLICK_POINT.y) }.expect("position cursor");
    let click = |flags| INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dwFlags: flags,
                ..Default::default()
            },
        },
    };
    assert_eq!(
        unsafe {
            SendInput(
                &[click(MOUSEEVENTF_LEFTDOWN), click(MOUSEEVENTF_LEFTUP)],
                std::mem::size_of::<INPUT>() as i32,
            )
        },
        2
    );
    pump();
    std::thread::sleep(std::time::Duration::from_millis(40));
    let clicks = TARGET_CLICKS.load(Ordering::Acquire);
    unsafe {
        let _ = ReleaseDC(None, dc);
        let _ = ShowWindow(window, SW_HIDE);
        let _ = DestroyWindow(window);
    }
    (pixel.0, hit == window, clicks)
}

unsafe extern "system" fn target_wnd_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_LBUTTONDOWN => {
            TARGET_CLICKS.fetch_add(1, Ordering::AcqRel);
            LRESULT(0)
        }
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn pump() {
    let mut message = MSG::default();
    while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

unsafe extern "system" fn probe_wnd_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCHITTEST {
        return LRESULT(HTTRANSPARENT as isize);
    }
    if message == WM_PAINT {
        unsafe {
            let mut paint = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut paint);
            let bitmap = vec![0x00ff00ffu32; (WIDTH * HEIGHT) as usize];
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: WIDTH,
                    biHeight: -HEIGHT,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let _ = SetDIBitsToDevice(
                dc,
                0,
                0,
                WIDTH as u32,
                HEIGHT as u32,
                0,
                0,
                0,
                HEIGHT as u32,
                bitmap.as_ptr().cast(),
                &info,
                DIB_RGB_COLORS,
            );
            let _ = EndPaint(hwnd, &paint);
        }
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}
