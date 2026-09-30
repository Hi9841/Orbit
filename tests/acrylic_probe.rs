//! Temporary desktop material probe. Ignored because it opens visible windows.
use std::time::Duration;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE, DwmExtendFrameIntoClientArea,
    DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, PAINTSTRUCT};
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, LWA_ALPHA, LWA_COLORKEY, MSG,
    PM_REMOVE, PeekMessageW, RegisterClassW, SW_HIDE, SW_SHOWNOACTIVATE,
    SetLayeredWindowAttributes, ShowWindow, TranslateMessage, WM_ERASEBKGND, WM_PAINT, WNDCLASSW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::w;

#[test]
#[ignore = "opens three temporary desktop material windows"]
fn compare_backdrop_variants() {
    let class = WNDCLASSW {
        lpfnWndProc: Some(probe_proc),
        lpszClassName: w!("OrbitAcrylicProbe"),
        ..Default::default()
    };
    assert_ne!(unsafe { RegisterClassW(&class) }, 0);
    let mut windows = Vec::new();
    for (index, style) in [
        WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
        WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_TRANSPARENT,
        WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_TRANSPARENT,
    ]
    .into_iter()
    .enumerate()
    {
        let window = unsafe {
            CreateWindowExW(
                style,
                w!("OrbitAcrylicProbe"),
                w!("Orbit acrylic probe"),
                WS_POPUP,
                60 + index as i32 * 280,
                60,
                240,
                190,
                None,
                None,
                None,
                None,
            )
        }
        .expect("create probe");
        let backdrop = DWMSBT_TRANSIENTWINDOW;
        let backdrop_result = unsafe {
            DwmSetWindowAttribute(
                window,
                DWMWA_SYSTEMBACKDROP_TYPE,
                (&backdrop as *const windows::Win32::Graphics::Dwm::DWM_SYSTEMBACKDROP_TYPE).cast(),
                std::mem::size_of_val(&backdrop) as u32,
            )
        };
        let frame_result = unsafe {
            DwmExtendFrameIntoClientArea(
                window,
                &MARGINS {
                    cxLeftWidth: -1,
                    cxRightWidth: -1,
                    cyTopHeight: -1,
                    cyBottomHeight: -1,
                },
            )
        };
        let layered_result = match index {
            1 => unsafe {
                SetLayeredWindowAttributes(
                    window,
                    COLORREF(0x0003_0201),
                    255,
                    LWA_COLORKEY | LWA_ALPHA,
                )
            },
            2 => unsafe { SetLayeredWindowAttributes(window, COLORREF(0), 170, LWA_ALPHA) },
            _ => Ok(()),
        };
        println!(
            "variant {index}: backdrop={backdrop_result:?}, frame={frame_result:?}, layered={layered_result:?}"
        );
        unsafe {
            let _ = ShowWindow(window, SW_SHOWNOACTIVATE);
        }
        windows.push(window);
    }
    for _ in 0..300 {
        let mut message = MSG::default();
        while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    for window in windows {
        unsafe {
            let _ = ShowWindow(window, SW_HIDE);
            let _ = DestroyWindow(window);
        }
    }
}

unsafe extern "system" fn probe_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_ERASEBKGND {
        return LRESULT(1);
    }
    if message == WM_PAINT {
        let mut paint = PAINTSTRUCT::default();
        unsafe {
            let _ = BeginPaint(hwnd, &mut paint);
            let _ = EndPaint(hwnd, &paint);
        }
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}
