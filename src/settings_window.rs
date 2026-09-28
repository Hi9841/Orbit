use crate::{configuration, platform};
use orbit::geometry::Action;
use orbit::settings::{
    CustomFrame, EdgePadding, Hotkey, PreviewStart, Settings, Shortcut, TriggerSide,
};
use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, COLOR_WINDOW, ClientToScreen, CreateFontW,
    CreateRectRgn, DEFAULT_CHARSET, DEFAULT_PITCH, DeleteObject, FF_DONTCARE, FW_NORMAL,
    FW_SEMIBOLD, GetMonitorInfoW, HBRUSH, HDC, HFONT, HGDIOBJ, InvalidateRect,
    MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow, OPAQUE, OUT_DEFAULT_PRECIS,
    SetBkMode, SetWindowRgn, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::Dialogs::{
    CommDlgExtendedError, GetOpenFileNameW, GetSaveFileNameW, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR,
    OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows::Win32::UI::Controls::SetScrollInfo;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, GetFocus, GetKeyState, SetFocus};
use windows::Win32::UI::WindowsAndMessaging::{
    BM_GETCHECK, BM_SETCHECK, BS_AUTOCHECKBOX, BS_DEFPUSHBUTTON, BS_GROUPBOX, BS_PUSHBUTTON,
    CB_ADDSTRING, CB_GETCURSEL, CB_GETITEMDATA, CB_SETCURSEL, CB_SETITEMDATA, CBS_DROPDOWNLIST,
    CBS_NOINTEGRALHEIGHT, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow,
    ES_AUTOVSCROLL, ES_MULTILINE, ES_NUMBER, ES_WANTRETURN, GetClassNameW, GetClientRect,
    GetDlgItem, GetParent, GetWindowTextLengthW, GetWindowTextW, HWND_BOTTOM, HWND_TOP, IDC_ARROW,
    IDI_APPLICATION, LB_ADDSTRING, LB_GETCURSEL, LB_SETCURSEL, LBN_SELCHANGE, LBS_NOTIFY,
    LoadCursorW, LoadIconW, MSG, RegisterClassW, SW_SHOW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SendMessageW, SetForegroundWindow, SetWindowPos, SetWindowTextW, ShowWindow, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_CREATE, WM_CTLCOLORBTN, WM_CTLCOLORSTATIC, WM_DESTROY,
    WM_DPICHANGED, WM_KEYDOWN, WM_SETFONT, WM_SIZE, WNDCLASSW, WS_BORDER, WS_CHILD,
    WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_OVERLAPPEDWINDOW, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GW_CHILD, GW_HWNDNEXT, GetScrollInfo, GetWindow, GetWindowRect, IsChild, SB_LINEDOWN,
    SB_LINEUP, SB_PAGEDOWN, SB_PAGEUP, SB_THUMBPOSITION, SB_THUMBTRACK, SB_VERT, SCROLLINFO,
    SIF_PAGE, SIF_POS, SIF_RANGE, SIF_TRACKPOS, SW_HIDE, SW_SHOWNA, WM_MOUSEWHEEL, WM_VSCROLL,
};
use windows::core::{PCWSTR, PWSTR, w};

const SIDEBAR: i32 = 10;
const PAGE_TITLE: i32 = 11;
const PAGE_DESCRIPTION: i32 = 12;
const STATUS: i32 = 13;
const SAVE: i32 = 14;
const CANCEL: i32 = 15;
const RESET: i32 = 16;
const IMPORT: i32 = 17;
const EXPORT: i32 = 18;
const PAGE_ID_START: i32 = 1000;

const BEHAVIOR_PADDING: i32 = 1000;
const BEHAVIOR_SIZE_INCREMENT: i32 = 1001;
const BEHAVIOR_SNAP_THRESHOLD: i32 = 1002;
const BEHAVIOR_STASH_PADDING: i32 = 1003;
const BEHAVIOR_SNAP: i32 = 1004;
const BEHAVIOR_SCREEN_CURSOR: i32 = 1005;
const BEHAVIOR_RESIZE_CURSOR: i32 = 1006;
const BEHAVIOR_FOCUS_RESIZE: i32 = 1007;
const BEHAVIOR_MOVE_CURSOR: i32 = 1008;
const BEHAVIOR_IGNORE_FULLSCREEN: i32 = 1009;
const BEHAVIOR_DISABLE_CURSOR: i32 = 1010;
const BEHAVIOR_TRIGGER_CONTROL: i32 = 1020;
const BEHAVIOR_TRIGGER_ALT: i32 = 1021;
const BEHAVIOR_TRIGGER_SHIFT: i32 = 1022;
const BEHAVIOR_TRIGGER_WIN: i32 = 1023;
const BEHAVIOR_TRIGGER_KEY: i32 = 1024;
const BEHAVIOR_TRIGGER_DELAY: i32 = 1025;
const BEHAVIOR_CYCLE_TIMEOUT: i32 = 1026;
const BEHAVIOR_CYCLE_SHIFT: i32 = 1027;
const BEHAVIOR_REVERSE_SCROLL: i32 = 1028;
const BEHAVIOR_LAUNCH_LOGIN: i32 = 1029;
const BEHAVIOR_UPDATES_ENABLED: i32 = 1030;
const BEHAVIOR_TRIGGER_ERROR: i32 = 1031;
const BEHAVIOR_LOCK_CENTER: i32 = 1032;

const RADIAL_VISIBLE: i32 = 1100;
const RADIAL_SIZE: i32 = 1101;
const RADIAL_THICKNESS: i32 = 1102;
const RADIAL_CORNER: i32 = 1103;
const RADIAL_COLOR: i32 = 1104;
const RADIAL_ERROR: i32 = 1105;
const RADIAL_SYSTEM_ACCENT: i32 = 1110;
const RADIAL_GRADIENT: i32 = 1111;
const RADIAL_GRADIENT_COLOR: i32 = 1112;
const RADIAL_ACTION_BASE: i32 = 1120;

const PREVIEW_VISIBLE: i32 = 1200;
const PREVIEW_OPACITY: i32 = 1201;
const PREVIEW_PADDING: i32 = 1202;
const PREVIEW_CORNER: i32 = 1203;
const PREVIEW_BORDER: i32 = 1204;
const PREVIEW_WINDOW_CORNERS: i32 = 1205;

const SHORTCUTS_ERROR: i32 = 1301;
const SHORTCUT_LIST: i32 = 1302;
const SHORTCUT_NEW: i32 = 1303;
const SHORTCUT_DELETE: i32 = 1304;
const SHORTCUT_CONTROL: i32 = 1310;
const SHORTCUT_ALT: i32 = 1311;
const SHORTCUT_SHIFT: i32 = 1312;
const SHORTCUT_WIN: i32 = 1313;
const SHORTCUT_KEY: i32 = 1314;
const SHORTCUT_CYCLE_LIST: i32 = 1320;
const SHORTCUT_ACTION_PICKER: i32 = 1321;
const SHORTCUT_ACTION_ADD: i32 = 1322;
const SHORTCUT_ACTION_REMOVE: i32 = 1323;
const SHORTCUT_APPLY: i32 = 1324;
const FRAMES_ERROR: i32 = 1401;
const FRAME_LIST: i32 = 1402;
const FRAME_NEW: i32 = 1403;
const FRAME_DELETE: i32 = 1404;
const FRAME_NAME: i32 = 1405;
const FRAME_X: i32 = 1406;
const FRAME_Y: i32 = 1407;
const FRAME_WIDTH: i32 = 1408;
const FRAME_HEIGHT: i32 = 1409;
const FRAME_APPLY: i32 = 1410;
const EXCLUSIONS_TEXT: i32 = 1500;
const ABOUT_UPDATE: i32 = 1600;
const ABOUT_STATUS: i32 = 1601;

const ADV_GROUP_PLACEMENT: i32 = 1800;
const ADV_GROUP_INPUT: i32 = 1801;
const ADV_EDGE_ENABLED: i32 = 1802;
const ADV_EDGE_TOP: i32 = 1803;
const ADV_EDGE_RIGHT: i32 = 1804;
const ADV_EDGE_BOTTOM: i32 = 1805;
const ADV_EDGE_LEFT: i32 = 1806;
const ADV_MIN_SCREEN_INCHES: i32 = 1807;
const ADV_ANIMATE_WINDOWS: i32 = 1808;
const ADV_ANIMATE_STASHED: i32 = 1809;
const ADV_ANIMATION_DURATION: i32 = 1810;
const ADV_RESTORE_ON_DRAG: i32 = 1811;
const ADV_SHIFT_FOCUS_STASHED: i32 = 1812;
const ADV_IGNORE_LOW_POWER: i32 = 1813;
const ADV_PREVIEW_START: i32 = 1814;
const ADV_CYCLE_RESTART: i32 = 1815;
const ADV_TRIGGER_SIDE: i32 = 1816;
const ADV_DOUBLE_TAP: i32 = 1817;
const ADV_MIDDLE_CLICK: i32 = 1818;
const ADV_MIDDLE_DELAY: i32 = 1819;
const ADV_TRIGGER_TIMEOUT: i32 = 1820;
const ADV_HIDE_NO_SELECTION: i32 = 1821;
const ADV_HIDE_TRAY: i32 = 1822;
const ADV_DEV_RELEASES: i32 = 1823;

const ACTION_LABEL_BASE: i32 = 3000;
const FIELD_ERROR_BASE: i32 = 4000;
const ACTION_COUNT: usize = 8;
const RADIAL_DIRECTIONS: [&str; ACTION_COUNT] = [
    "Right",
    "Bottom right",
    "Bottom",
    "Bottom left",
    "Left",
    "Top left",
    "Top",
    "Top right",
];
const PAGE_NAMES: [&str; 8] = [
    "General & behavior",
    "Radial menu",
    "Preview",
    "Shortcuts",
    "Custom frames",
    "Exclusions",
    "Advanced",
    "About & updates",
];

#[derive(Clone)]
struct WindowState {
    settings: Settings,
    page: usize,
    status: String,
    editing_shortcut: Option<usize>,
    shortcut_actions: Vec<Action>,
    editing_frame: Option<usize>,
    scroll_offset: i32,
}

#[derive(Clone, Copy)]
struct WindowFonts {
    body: HFONT,
    title: HFONT,
}

thread_local! {
    static WINDOW: Cell<Option<HWND>> = const { Cell::new(None) };
    static STATE: RefCell<Option<WindowState>> = const { RefCell::new(None) };
    static FONTS: Cell<Option<WindowFonts>> = const { Cell::new(None) };
}

pub fn open() -> Result<(), String> {
    if let Some(hwnd) = WINDOW.with(Cell::get) {
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = SetForegroundWindow(hwnd);
        }
        return Ok(());
    }

    let (settings, status) = match Settings::load() {
        Ok(settings) => (settings, String::new()),
        Err(error) => (
            Settings::default(),
            format!("Settings could not be loaded: {error}"),
        ),
    };
    STATE.with(|state| {
        *state.borrow_mut() = Some(WindowState {
            settings,
            page: 0,
            status,
            editing_shortcut: None,
            shortcut_actions: Vec::new(),
            editing_frame: None,
            scroll_offset: 0,
        });
    });

    let module = unsafe { GetModuleHandleW(None) }.map_err(|error| error.to_string())?;
    let instance = HINSTANCE(module.0);
    let class = WNDCLASSW {
        lpfnWndProc: Some(settings_proc),
        hInstance: instance,
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.map_err(|error| error.to_string())?,
        hIcon: unsafe { LoadIconW(Some(instance), PCWSTR(std::ptr::without_provenance(1))) }
            .unwrap_or(
                unsafe { LoadIconW(None, IDI_APPLICATION) }.map_err(|error| error.to_string())?,
            ),
        lpszClassName: w!("OrbitSettings"),
        hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as usize as *mut _),
        ..Default::default()
    };
    let _ = unsafe { RegisterClassW(&class) };
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("OrbitSettings"),
            w!("Orbit settings"),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN | WS_VSCROLL,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1020,
            740,
            None,
            None,
            Some(instance),
            None,
        )
    }
    .map_err(|error| {
        STATE.with(|state| *state.borrow_mut() = None);
        error.to_string()
    })?;
    WINDOW.with(|slot| slot.set(Some(hwnd)));
    fit_window_to_work_area(hwnd, 1020, 740);
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
    Ok(())
}

pub fn handle_dialog_message(message: &MSG) -> bool {
    WINDOW.with(Cell::get).is_some_and(|hwnd| {
        if message.message == WM_KEYDOWN && unsafe { GetKeyState(0x11) } < 0 {
            match message.wParam.0 {
                0x21 => {
                    scroll_page(hwnd, SB_PAGEUP.0);
                    return true;
                }
                0x22 => {
                    scroll_page(hwnd, SB_PAGEDOWN.0);
                    return true;
                }
                0x24 => {
                    set_scroll_offset(hwnd, 0);
                    return true;
                }
                0x23 => {
                    set_scroll_offset(hwnd, i32::MAX);
                    return true;
                }
                _ => {}
            }
        }
        let handled = unsafe {
            windows::Win32::UI::WindowsAndMessaging::IsDialogMessageW(hwnd, message).as_bool()
        };
        if handled && message.message == WM_KEYDOWN {
            ensure_focused_control_visible(hwnd);
        }
        handled
    })
}

fn ensure_focused_control_visible(hwnd: HWND) {
    let focus = unsafe { GetFocus() };
    if focus.0.is_null() || !unsafe { IsChild(hwnd, focus) }.as_bool() {
        return;
    }
    let mut control_rect = RECT::default();
    let mut client_origin = POINT::default();
    if unsafe { GetWindowRect(focus, &mut control_rect) }.is_err()
        || !unsafe { ClientToScreen(hwnd, &mut client_origin) }.as_bool()
    {
        return;
    }
    let dpi = dpi_for_window(hwnd);
    let (_, height) = client_size_logical(hwnd);
    let viewport_top = px(108, dpi);
    let viewport_bottom = px(height - 112, dpi);
    let control_left = control_rect.left - client_origin.x;
    if control_left < px(220, dpi) {
        return;
    }
    let control_top = control_rect.top - client_origin.y;
    let control_bottom = control_rect.bottom - client_origin.y;
    let delta = if control_top < viewport_top {
        control_top - viewport_top
    } else if control_bottom > viewport_bottom {
        control_bottom - viewport_bottom
    } else {
        return;
    };
    let logical_delta = ((i64::from(delta) * 96) / i64::from(dpi)) as i32;
    let current = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map_or(0, |state| state.scroll_offset)
    });
    set_scroll_offset(hwnd, current + logical_delta);
}

unsafe extern "system" fn settings_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CREATE => {
            rebuild_fonts(hwnd);
            if let Err(error) = create_shell(hwnd) {
                set_status(hwnd, &error);
                return LRESULT(-1);
            }
            fill_sidebar(hwnd);
            let page = STATE.with(|state| state.borrow().as_ref().map_or(0, |state| state.page));
            if let Err(error) = build_page(hwnd, page) {
                set_status(hwnd, &error);
            }
            layout_window(hwnd);
            update_caption(hwnd, page);
            refresh_status(hwnd);
            LRESULT(0)
        }
        WM_SIZE => {
            layout_window(hwnd);
            let page = STATE.with(|state| state.borrow().as_ref().map_or(0, |state| state.page));
            layout_page(hwnd, page);
            LRESULT(0)
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            let hdc = HDC(wparam.0 as *mut _);
            let control = HWND(lparam.0 as *mut _);
            // The combo's selection field is a child static. Transparent text redraws
            // on top of the previous glyphs and leaves a stale first character.
            let background = if parent_is_class(control, "ComboBox") {
                OPAQUE
            } else {
                TRANSPARENT
            };
            unsafe {
                let _ = SetBkMode(hdc, background);
            }
            LRESULT(HBRUSH((COLOR_WINDOW.0 + 1) as usize as *mut _).0 as isize)
        }
        WM_DPICHANGED => {
            if lparam.0 != 0 {
                let rect = unsafe { &*(lparam.0 as *const RECT) };
                let _ = unsafe {
                    SetWindowPos(
                        hwnd,
                        None,
                        rect.left,
                        rect.top,
                        rect.right - rect.left,
                        rect.bottom - rect.top,
                        windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER
                            | windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE,
                    )
                };
            }
            rebuild_fonts(hwnd);
            layout_window(hwnd);
            let page = STATE.with(|state| state.borrow().as_ref().map_or(0, |state| state.page));
            layout_page(hwnd, page);
            LRESULT(0)
        }
        WM_VSCROLL => {
            scroll_page(hwnd, (wparam.0 & 0xffff) as i32);
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam.0 >> 16) & 0xffff) as u16 as i16 as i32;
            scroll_by(
                hwnd,
                -delta.signum() * 40 * (delta.unsigned_abs() as i32 / 120).max(1),
            );
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xffff) as i32;
            let code = ((wparam.0 >> 16) & 0xffff) as u32;
            match id {
                SIDEBAR if code == LBN_SELCHANGE => switch_page(hwnd),
                SHORTCUT_LIST if code == LBN_SELCHANGE => select_shortcut(hwnd),
                FRAME_LIST if code == LBN_SELCHANGE => select_frame(hwnd),
                SHORTCUT_NEW => new_shortcut(hwnd),
                SHORTCUT_DELETE => delete_shortcut(hwnd),
                SHORTCUT_ACTION_ADD => add_shortcut_action(hwnd),
                SHORTCUT_ACTION_REMOVE => remove_shortcut_action(hwnd),
                SHORTCUT_APPLY => apply_shortcut_editor(hwnd),
                RADIAL_SYSTEM_ACCENT => update_radial_color_enabled(hwnd),
                RADIAL_GRADIENT => update_gradient_enabled(hwnd),
                ADV_EDGE_ENABLED => update_edge_padding_enabled(hwnd),
                ADV_ANIMATE_WINDOWS | ADV_ANIMATE_STASHED => update_animation_enabled(hwnd),
                ADV_MIDDLE_CLICK => update_middle_click_delay_enabled(hwnd),
                FRAME_NEW => new_frame(hwnd),
                FRAME_DELETE => delete_frame(hwnd),
                FRAME_APPLY => apply_frame_editor(hwnd),
                SAVE => save_settings(hwnd),
                CANCEL => unsafe {
                    let _ = DestroyWindow(hwnd);
                },
                RESET => reset_editor(hwnd),
                IMPORT => import_settings(hwnd),
                EXPORT => export_settings(hwnd),
                ABOUT_UPDATE => check_for_updates(hwnd),
                _ => {}
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            WINDOW.with(|slot| slot.set(None));
            STATE.with(|state| *state.borrow_mut() = None);
            delete_fonts();
            LRESULT(0)
        }
        windows::Win32::UI::WindowsAndMessaging::WM_GETMINMAXINFO => {
            if lparam.0 != 0 {
                let info = unsafe {
                    &mut *(lparam.0 as *mut windows::Win32::UI::WindowsAndMessaging::MINMAXINFO)
                };
                let dpi = dpi_for_window(hwnd);
                let work = monitor_work_area(hwnd);
                info.ptMinTrackSize.x =
                    px(900, dpi).min((work.right - work.left - px(24, dpi)).max(1));
                info.ptMinTrackSize.y =
                    px(650, dpi).min((work.bottom - work.top - px(56, dpi)).max(1));
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn rebuild_fonts(hwnd: HWND) {
    let dpi = dpi_for_window(hwnd);
    let body_height = -((10_i64 * i64::from(dpi) + 36) / 72) as i32;
    let title_height = -((15_i64 * i64::from(dpi) + 36) / 72) as i32;
    let body = unsafe {
        CreateFontW(
            body_height,
            0,
            0,
            0,
            FW_NORMAL.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            u32::from(DEFAULT_PITCH.0) | u32::from(FF_DONTCARE.0),
            w!("Segoe UI"),
        )
    };
    let title = unsafe {
        CreateFontW(
            title_height,
            0,
            0,
            0,
            FW_SEMIBOLD.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            u32::from(DEFAULT_PITCH.0) | u32::from(FF_DONTCARE.0),
            w!("Segoe UI"),
        )
    };
    let fonts = WindowFonts { body, title };
    let old = FONTS.with(|slot| slot.replace(Some(fonts)));
    let mut child = unsafe { GetWindow(hwnd, GW_CHILD).ok() };
    while let Some(current) = child {
        let id = unsafe { windows::Win32::UI::WindowsAndMessaging::GetDlgCtrlID(current) };
        let font = if id == PAGE_TITLE { title } else { body };
        unsafe {
            SendMessageW(
                current,
                WM_SETFONT,
                Some(WPARAM(font.0 as usize)),
                Some(LPARAM(1)),
            );
        }
        child = unsafe { GetWindow(current, GW_HWNDNEXT).ok() };
    }
    if let Some(old) = old {
        unsafe {
            if !old.body.0.is_null() {
                let _ = DeleteObject(HGDIOBJ(old.body.0));
            }
            if !old.title.0.is_null() {
                let _ = DeleteObject(HGDIOBJ(old.title.0));
            }
        }
    }
}

fn delete_fonts() {
    if let Some(fonts) = FONTS.with(|slot| slot.take()) {
        unsafe {
            if !fonts.body.0.is_null() {
                let _ = DeleteObject(HGDIOBJ(fonts.body.0));
            }
            if !fonts.title.0.is_null() {
                let _ = DeleteObject(HGDIOBJ(fonts.title.0));
            }
        }
    }
}

fn monitor_work_area(hwnd: HWND) -> RECT {
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if unsafe { GetMonitorInfoW(monitor, &mut info).as_bool() } {
        info.rcWork
    } else {
        RECT {
            left: 0,
            top: 0,
            right: 1280,
            bottom: 720,
        }
    }
}

fn fit_window_to_work_area(hwnd: HWND, desired_width: i32, desired_height: i32) {
    let dpi = dpi_for_window(hwnd);
    let work = monitor_work_area(hwnd);
    let work_width = work.right - work.left;
    let work_height = work.bottom - work.top;
    let width = px(desired_width, dpi).min((work_width - px(24, dpi)).max(1));
    let height = px(desired_height, dpi).min((work_height - px(32, dpi)).max(1));
    let x = work.left + (work_width - width) / 2;
    let y = work.top + (work_height - height) / 2;
    let _ = unsafe {
        SetWindowPos(
            hwnd,
            None,
            x,
            y,
            width,
            height,
            windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER
                | windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE,
        )
    };
}

fn content_extent(page: usize, height: i32) -> i32 {
    match page {
        0 => (height - 220).max(474),
        3 => (height - 220).max(430),
        4 => (height - 220).max(404),
        5 => (height - 220).max(296),
        1 => (height - 212).max(450),
        2 => (height - 220).max(470),
        6 => (height - 220).max(560),
        7 => 238,
        _ => 0,
    }
}

fn update_scrollbar(hwnd: HWND, page: usize, height: i32) {
    let viewport = (height - 220).max(1);
    let extent = content_extent(page, height);
    let max_scroll = (extent - viewport).max(0);
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.scroll_offset = state.scroll_offset.clamp(0, max_scroll);
        }
    });
    let position = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map_or(0, |state| state.scroll_offset)
    });
    let info = SCROLLINFO {
        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: SIF_RANGE | SIF_PAGE | SIF_POS,
        nMin: 0,
        nMax: extent.saturating_sub(1),
        nPage: viewport as u32,
        nPos: position,
        ..Default::default()
    };
    let _ = unsafe { SetScrollInfo(hwnd, SB_VERT, &info, true) };
}

fn scroll_page(hwnd: HWND, command: i32) {
    let (_, height) = client_size_logical(hwnd);
    let viewport = (height - 220).max(1);
    let mut info = SCROLLINFO {
        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: SIF_TRACKPOS,
        ..Default::default()
    };
    let _ = unsafe { GetScrollInfo(hwnd, SB_VERT, &mut info) };
    let current = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map_or(0, |state| state.scroll_offset)
    });
    let next = match command {
        value if value == SB_LINEUP.0 => current - 40,
        value if value == SB_LINEDOWN.0 => current + 40,
        value if value == SB_PAGEUP.0 => current - viewport,
        value if value == SB_PAGEDOWN.0 => current + viewport,
        value if value == SB_THUMBTRACK.0 || value == SB_THUMBPOSITION.0 => info.nTrackPos,
        _ => current,
    };
    set_scroll_offset(hwnd, next);
}

fn scroll_by(hwnd: HWND, delta: i32) {
    let current = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map_or(0, |state| state.scroll_offset)
    });
    set_scroll_offset(hwnd, current + delta);
}

fn set_scroll_offset(hwnd: HWND, requested: i32) {
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.scroll_offset = requested.max(0);
        }
    });
    layout_page(hwnd, current_page());
}

fn create_shell(hwnd: HWND) -> Result<(), String> {
    child(
        hwnd,
        w!("LISTBOX"),
        w!(""),
        WINDOW_STYLE(LBS_NOTIFY as u32) | WS_BORDER | WS_TABSTOP,
        SIDEBAR,
    )?;
    child(hwnd, w!("STATIC"), w!(""), WINDOW_STYLE(0x80), PAGE_TITLE)?;
    child(
        hwnd,
        w!("STATIC"),
        w!(""),
        WINDOW_STYLE(0),
        PAGE_DESCRIPTION,
    )?;
    child(hwnd, w!("STATIC"), w!(""), WINDOW_STYLE(0), STATUS)?;
    child(
        hwnd,
        w!("BUTTON"),
        w!("Save"),
        WS_TABSTOP | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
        SAVE,
    )?;
    child(
        hwnd,
        w!("BUTTON"),
        w!("Cancel"),
        WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        CANCEL,
    )?;
    child(
        hwnd,
        w!("BUTTON"),
        w!("Reset defaults"),
        WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        RESET,
    )?;
    child(
        hwnd,
        w!("BUTTON"),
        w!("Import…"),
        WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        IMPORT,
    )?;
    child(
        hwnd,
        w!("BUTTON"),
        w!("Export…"),
        WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        EXPORT,
    )?;
    Ok(())
}

fn fill_sidebar(hwnd: HWND) {
    let Ok(sidebar) = (unsafe { GetDlgItem(Some(hwnd), SIDEBAR) }) else {
        return;
    };
    for title in PAGE_NAMES {
        send_text(sidebar, LB_ADDSTRING, title);
    }
    unsafe {
        SendMessageW(sidebar, LB_SETCURSEL, Some(WPARAM(0)), None);
    }
}

fn child(
    parent: HWND,
    class: PCWSTR,
    text: PCWSTR,
    style: WINDOW_STYLE,
    id: i32,
) -> Result<HWND, String> {
    let instance = unsafe { GetModuleHandleW(None) }.map_err(|error| error.to_string())?;
    let control = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            text,
            WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | style,
            0,
            0,
            1,
            1,
            Some(parent),
            Some(windows::Win32::UI::WindowsAndMessaging::HMENU(
                id as usize as *mut _,
            )),
            Some(HINSTANCE(instance.0)),
            None,
        )
    }
    .map_err(|error| error.to_string())?;
    unsafe {
        let fonts = FONTS.with(Cell::get);
        let font = if id == PAGE_TITLE {
            fonts.map_or(HFONT::default(), |fonts| fonts.title)
        } else {
            fonts.map_or(HFONT::default(), |fonts| fonts.body)
        };
        if !font.0.is_null() {
            SendMessageW(
                control,
                WM_SETFONT,
                Some(WPARAM(font.0 as usize)),
                Some(LPARAM(1)),
            );
        }
    }
    Ok(control)
}

fn create_text(parent: HWND, id: i32, text: &str) -> Result<HWND, String> {
    let wide = to_wide(text);
    child(
        parent,
        w!("STATIC"),
        PCWSTR(wide.as_ptr()),
        WINDOW_STYLE(0x80), // SS_NOPREFIX: show literal ampersands in labels and page names.
        id,
    )
}

fn create_button(parent: HWND, id: i32, text: &str) -> Result<HWND, String> {
    let wide = to_wide(text);
    child(
        parent,
        w!("BUTTON"),
        PCWSTR(wide.as_ptr()),
        WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        id,
    )
}

fn create_checkbox(parent: HWND, id: i32, text: &str, checked: bool) -> Result<HWND, String> {
    let wide = to_wide(text);
    let control = child(
        parent,
        w!("BUTTON"),
        PCWSTR(wide.as_ptr()),
        WS_TABSTOP | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
        id,
    )?;
    set_checked(control, checked);
    Ok(control)
}

fn create_numeric(parent: HWND, id: i32, title: &str, value: i64) -> Result<(), String> {
    create_text(parent, id + 1000, title)?;
    let value = value.to_string();
    let wide = to_wide(&value);
    child(
        parent,
        w!("EDIT"),
        PCWSTR(wide.as_ptr()),
        WS_BORDER | WS_TABSTOP | WINDOW_STYLE(ES_NUMBER as u32),
        id,
    )?;
    create_text(parent, field_error_id(id), "")?;
    Ok(())
}

fn create_decimal(parent: HWND, id: i32, title: &str, value: f64) -> Result<(), String> {
    create_text(parent, id + 1000, title)?;
    let wide = to_wide(&format_float(value));
    child(
        parent,
        w!("EDIT"),
        PCWSTR(wide.as_ptr()),
        WS_BORDER | WS_TABSTOP,
        id,
    )?;
    create_text(parent, field_error_id(id), "")?;
    Ok(())
}

fn create_edit(parent: HWND, id: i32, text: &str, multiline: bool) -> Result<(), String> {
    let wide = to_wide(text);
    let mut style = WS_BORDER | WS_TABSTOP;
    if multiline {
        style |= WS_VSCROLL
            | WINDOW_STYLE(ES_MULTILINE as u32 | ES_AUTOVSCROLL as u32 | ES_WANTRETURN as u32);
    } else {
        style |= WINDOW_STYLE(windows::Win32::UI::WindowsAndMessaging::ES_AUTOHSCROLL as u32);
    }
    child(parent, w!("EDIT"), PCWSTR(wide.as_ptr()), style, id)?;
    Ok(())
}

fn create_combo(parent: HWND, id: i32) -> Result<HWND, String> {
    child(
        parent,
        w!("COMBOBOX"),
        w!(""),
        WS_TABSTOP
            | WS_VSCROLL
            | WINDOW_STYLE(CBS_DROPDOWNLIST as u32 | CBS_NOINTEGRALHEIGHT as u32),
        id,
    )
}

fn create_list(parent: HWND, id: i32) -> Result<HWND, String> {
    child(
        parent,
        w!("LISTBOX"),
        w!(""),
        WS_BORDER | WS_TABSTOP | WS_VSCROLL | WINDOW_STYLE(LBS_NOTIFY as u32),
        id,
    )
}

fn create_group(parent: HWND, id: i32, title: &str) -> Result<(), String> {
    let wide = to_wide(title);
    child(
        parent,
        w!("BUTTON"),
        PCWSTR(wide.as_ptr()),
        WINDOW_STYLE(BS_GROUPBOX as u32),
        id,
    )?;
    Ok(())
}

fn build_page(hwnd: HWND, page: usize) -> Result<(), String> {
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.scroll_offset = 0;
        }
    });
    destroy_page_controls(hwnd);
    let settings = STATE
        .with(|state| state.borrow().as_ref().map(|state| state.settings.clone()))
        .ok_or_else(|| "settings window state is unavailable".to_string())?;
    match page {
        0 => build_behavior(hwnd, &settings)?,
        1 => build_radial(hwnd, &settings)?,
        2 => build_preview(hwnd, &settings)?,
        3 => build_shortcuts(hwnd, &settings)?,
        4 => build_custom_frames(hwnd, &settings)?,
        5 => build_exclusions(hwnd, &settings)?,
        6 => build_advanced(hwnd, &settings)?,
        7 => build_about(hwnd)?,
        _ => return Err("unknown settings page".into()),
    }
    layout_page(hwnd, page);
    update_caption(hwnd, page);
    Ok(())
}

fn destroy_page_controls(hwnd: HWND) {
    for id in PAGE_ID_START..=FIELD_ERROR_BASE + 2000 {
        if let Ok(control) = unsafe { GetDlgItem(Some(hwnd), id) } {
            unsafe {
                let _ = DestroyWindow(control);
            }
        }
    }
}

fn build_behavior(hwnd: HWND, settings: &Settings) -> Result<(), String> {
    create_group(hwnd, 1700, "Window placement")?;
    create_numeric(
        hwnd,
        BEHAVIOR_PADDING,
        "Window padding (px)",
        i64::from(settings.padding),
    )?;
    create_numeric(
        hwnd,
        BEHAVIOR_SIZE_INCREMENT,
        "Resize step (px)",
        i64::from(settings.size_increment),
    )?;
    create_numeric(
        hwnd,
        BEHAVIOR_SNAP_THRESHOLD,
        "Snap threshold (px)",
        i64::from(settings.snap_threshold),
    )?;
    create_numeric(
        hwnd,
        BEHAVIOR_STASH_PADDING,
        "Visible stash edge (px)",
        i64::from(settings.stash_visible_padding),
    )?;
    create_checkbox(
        hwnd,
        BEHAVIOR_SNAP,
        "Snap windows while dragging",
        settings.snap_on_drag,
    )?;
    create_checkbox(
        hwnd,
        BEHAVIOR_SCREEN_CURSOR,
        "Use the monitor under the cursor",
        settings.use_screen_with_cursor,
    )?;
    create_checkbox(
        hwnd,
        BEHAVIOR_RESIZE_CURSOR,
        "Resize the window under the cursor",
        settings.resize_window_under_cursor,
    )?;
    create_checkbox(
        hwnd,
        BEHAVIOR_FOCUS_RESIZE,
        "Focus the window after resizing",
        settings.focus_window_on_resize,
    )?;
    create_checkbox(
        hwnd,
        BEHAVIOR_MOVE_CURSOR,
        "Move the cursor with the resized window",
        settings.move_cursor_with_window,
    )?;
    create_checkbox(
        hwnd,
        BEHAVIOR_IGNORE_FULLSCREEN,
        "Ignore full-screen windows",
        settings.ignore_fullscreen,
    )?;
    create_checkbox(
        hwnd,
        BEHAVIOR_DISABLE_CURSOR,
        "Disable radial cursor interaction",
        settings.disable_cursor_interaction,
    )?;
    create_checkbox(
        hwnd,
        BEHAVIOR_LOCK_CENTER,
        "Keep the radial menu centered on its target",
        settings.lock_radial_menu_to_center,
    )?;

    create_group(hwnd, 1701, "Trigger and timing")?;
    create_checkbox(
        hwnd,
        BEHAVIOR_TRIGGER_CONTROL,
        "Control",
        settings.trigger.control,
    )?;
    create_checkbox(hwnd, BEHAVIOR_TRIGGER_ALT, "Alt", settings.trigger.alt)?;
    create_checkbox(
        hwnd,
        BEHAVIOR_TRIGGER_SHIFT,
        "Shift",
        settings.trigger.shift,
    )?;
    create_checkbox(hwnd, BEHAVIOR_TRIGGER_WIN, "Windows", settings.trigger.win)?;
    create_text(hwnd, BEHAVIOR_TRIGGER_KEY + 1000, "Trigger key")?;
    let combo = create_combo(hwnd, BEHAVIOR_TRIGGER_KEY)?;
    fill_key_combo(combo, settings.trigger.key);
    create_text(hwnd, BEHAVIOR_TRIGGER_ERROR, "")?;
    create_numeric(
        hwnd,
        BEHAVIOR_TRIGGER_DELAY,
        "Trigger delay (ms)",
        i64::from(settings.trigger_delay_ms),
    )?;
    create_numeric(
        hwnd,
        BEHAVIOR_CYCLE_TIMEOUT,
        "Cycle timeout (ms)",
        i64::from(settings.cycle_timeout_ms),
    )?;
    create_checkbox(
        hwnd,
        BEHAVIOR_CYCLE_SHIFT,
        "Reverse cycles while holding Shift",
        settings.cycle_backwards_on_shift,
    )?;
    create_checkbox(
        hwnd,
        BEHAVIOR_REVERSE_SCROLL,
        "Reverse scroll direction",
        settings.reverse_scroll,
    )?;
    create_checkbox(
        hwnd,
        BEHAVIOR_LAUNCH_LOGIN,
        "Launch Orbit when I sign in",
        settings.launch_at_login,
    )?;
    create_checkbox(
        hwnd,
        BEHAVIOR_UPDATES_ENABLED,
        "Check for updates automatically",
        settings.updates_enabled,
    )?;
    Ok(())
}

fn build_radial(hwnd: HWND, settings: &Settings) -> Result<(), String> {
    create_group(hwnd, 1710, "Radial menu")?;
    create_checkbox(
        hwnd,
        RADIAL_VISIBLE,
        "Show the radial menu",
        settings.radial_menu_visible,
    )?;
    create_numeric(
        hwnd,
        RADIAL_SIZE,
        "Ring diameter (pt)",
        i64::from(settings.radial_size),
    )?;
    create_numeric(
        hwnd,
        RADIAL_THICKNESS,
        "Ring thickness (pt)",
        i64::from(settings.radial_thickness),
    )?;
    create_numeric(
        hwnd,
        RADIAL_CORNER,
        "Corner radius (pt)",
        i64::from(settings.radial_corner_radius),
    )?;
    create_text(hwnd, RADIAL_COLOR + 1000, "Accent color (#RRGGBB)")?;
    let color = format!("#{:06X}", settings.accent_color & 0x00ff_ffff);
    create_edit(hwnd, RADIAL_COLOR, &color, false)?;
    create_text(hwnd, field_error_id(RADIAL_COLOR), "")?;
    create_checkbox(
        hwnd,
        RADIAL_SYSTEM_ACCENT,
        "Use Windows accent color",
        settings.use_system_accent,
    )?;
    create_checkbox(
        hwnd,
        RADIAL_GRADIENT,
        "Use a color gradient",
        settings.use_gradient,
    )?;
    create_text(
        hwnd,
        RADIAL_GRADIENT_COLOR + 1000,
        "Gradient end color (#RRGGBB)",
    )?;
    let gradient_color = format!("#{:06X}", settings.gradient_color & 0x00ff_ffff);
    create_edit(hwnd, RADIAL_GRADIENT_COLOR, &gradient_color, false)?;
    create_text(hwnd, field_error_id(RADIAL_GRADIENT_COLOR), "")?;
    update_radial_color_enabled(hwnd);
    update_gradient_enabled(hwnd);
    let options = action_options(settings);
    for (slot, direction) in RADIAL_DIRECTIONS.iter().enumerate() {
        let id = RADIAL_ACTION_BASE + slot as i32;
        create_text(hwnd, ACTION_LABEL_BASE + slot as i32, direction)?;
        let combo = create_combo(hwnd, id)?;
        for action in &options {
            let label = action_label(*action, settings);
            send_text(combo, CB_ADDSTRING, &label);
        }
        let selected = options
            .iter()
            .position(|action| *action == settings.radial_actions[slot])
            .unwrap_or(0);
        unsafe {
            SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(selected)), None);
        }
    }
    create_text(hwnd, RADIAL_ERROR, "")?;
    Ok(())
}

fn build_preview(hwnd: HWND, settings: &Settings) -> Result<(), String> {
    create_group(hwnd, 1720, "Window preview")?;
    create_checkbox(
        hwnd,
        PREVIEW_VISIBLE,
        "Show the preview while choosing a layout",
        settings.preview_visible,
    )?;
    create_numeric(
        hwnd,
        PREVIEW_OPACITY,
        "Preview opacity (0-255)",
        i64::from(settings.preview_opacity),
    )?;
    create_numeric(
        hwnd,
        PREVIEW_PADDING,
        "Preview inset (px)",
        i64::from(settings.preview_padding),
    )?;
    create_numeric(
        hwnd,
        PREVIEW_CORNER,
        "Fallback corner radius (96-DPI px)",
        i64::from(settings.preview_corner_radius),
    )?;
    create_numeric(
        hwnd,
        PREVIEW_BORDER,
        "Border thickness (96-DPI px)",
        i64::from(settings.preview_border_thickness),
    )?;
    create_checkbox(
        hwnd,
        PREVIEW_WINDOW_CORNERS,
        "Use the standard Windows corner radius",
        settings.preview_use_window_corner_radius,
    )?;
    create_text(
        hwnd,
        1721,
        "The selected frame sets preview size. Border and fallback radius scale with DPI; the per-pixel radius stays active on Windows versions without DWM rounded corners.",
    )?;
    Ok(())
}

fn build_advanced(hwnd: HWND, settings: &Settings) -> Result<(), String> {
    create_group(hwnd, ADV_GROUP_PLACEMENT, "Placement, edges, and animation")?;
    create_decimal(
        hwnd,
        ADV_MIN_SCREEN_INCHES,
        "Minimum screen diagonal (inches; 0 disables)",
        settings.padding_minimum_screen_inches,
    )?;
    let edge = settings.edge_padding.unwrap_or_default();
    create_checkbox(
        hwnd,
        ADV_EDGE_ENABLED,
        "Set padding separately for each screen edge",
        settings.edge_padding.is_some(),
    )?;
    create_numeric(
        hwnd,
        ADV_EDGE_TOP,
        "Top edge padding (px)",
        i64::from(edge.top),
    )?;
    create_numeric(
        hwnd,
        ADV_EDGE_RIGHT,
        "Right edge padding (px)",
        i64::from(edge.right),
    )?;
    create_numeric(
        hwnd,
        ADV_EDGE_BOTTOM,
        "Bottom edge padding (px)",
        i64::from(edge.bottom),
    )?;
    create_numeric(
        hwnd,
        ADV_EDGE_LEFT,
        "Left edge padding (px)",
        i64::from(edge.left),
    )?;
    create_checkbox(
        hwnd,
        ADV_RESTORE_ON_DRAG,
        "Restore the previous frame when dragging a window",
        settings.restore_window_frame_on_drag,
    )?;
    create_checkbox(
        hwnd,
        ADV_SHIFT_FOCUS_STASHED,
        "Move focus to the next window when stashing",
        settings.shift_focus_when_stashed,
    )?;
    create_checkbox(
        hwnd,
        ADV_ANIMATE_WINDOWS,
        "Animate window resizing",
        settings.animate_window_resizes,
    )?;
    create_checkbox(
        hwnd,
        ADV_ANIMATE_STASHED,
        "Animate stashing and restoring",
        settings.animate_stashed_windows,
    )?;
    create_numeric(
        hwnd,
        ADV_ANIMATION_DURATION,
        "Animation duration (ms)",
        i64::from(settings.animation_duration_ms),
    )?;
    create_checkbox(
        hwnd,
        ADV_IGNORE_LOW_POWER,
        "Allow animations in low-power mode",
        settings.ignore_low_power_mode,
    )?;

    create_group(hwnd, ADV_GROUP_INPUT, "Input, cycling, and updates")?;
    create_text(hwnd, ADV_PREVIEW_START + 1000, "Start preview from")?;
    let preview_start = create_combo(hwnd, ADV_PREVIEW_START)?;
    for label in [
        "Selected action",
        "Center of the radial menu",
        "Center of the screen",
    ] {
        send_text(preview_start, CB_ADDSTRING, label);
    }
    set_combo_selection(
        preview_start,
        match settings.preview_start {
            PreviewStart::ActionCenter => 0,
            PreviewStart::RadialMenu => 1,
            PreviewStart::ScreenCenter => 2,
        },
    );
    create_text(hwnd, ADV_TRIGGER_SIDE + 1000, "Trigger modifier side")?;
    let trigger_side = create_combo(hwnd, ADV_TRIGGER_SIDE)?;
    for label in ["Either side", "Left button", "Right button"] {
        send_text(trigger_side, CB_ADDSTRING, label);
    }
    set_combo_selection(
        trigger_side,
        match settings.trigger_side {
            TriggerSide::Either => 0,
            TriggerSide::Left => 1,
            TriggerSide::Right => 2,
        },
    );
    create_checkbox(
        hwnd,
        ADV_CYCLE_RESTART,
        "Restart cycles after another action",
        settings.cycle_restart,
    )?;
    create_checkbox(
        hwnd,
        ADV_DOUBLE_TAP,
        "Double-tap the trigger to activate it",
        settings.double_tap_to_trigger,
    )?;
    create_checkbox(
        hwnd,
        ADV_MIDDLE_CLICK,
        "Allow middle-click as a trigger",
        settings.middle_click_triggers,
    )?;
    create_checkbox(
        hwnd,
        ADV_MIDDLE_DELAY,
        "Use the trigger delay for middle-click",
        settings.middle_click_uses_delay,
    )?;
    create_numeric(
        hwnd,
        ADV_TRIGGER_TIMEOUT,
        "Trigger timeout (ms; 0 disables)",
        i64::from(settings.trigger_timeout_ms),
    )?;
    create_checkbox(
        hwnd,
        ADV_HIDE_NO_SELECTION,
        "Hide the menu when nothing is selected",
        settings.hide_on_no_selection,
    )?;
    create_checkbox(
        hwnd,
        ADV_HIDE_TRAY,
        "Hide the notification-area icon",
        settings.hide_tray_icon,
    )?;
    create_checkbox(
        hwnd,
        ADV_DEV_RELEASES,
        "Include development updates",
        settings.include_development_versions,
    )?;
    update_edge_padding_enabled(hwnd);
    update_animation_enabled(hwnd);
    update_middle_click_delay_enabled(hwnd);
    Ok(())
}

fn build_shortcuts(hwnd: HWND, settings: &Settings) -> Result<(), String> {
    create_group(hwnd, 1730, "Keyboard shortcuts and action cycles")?;
    create_text(
        hwnd,
        1731,
        "Choose a shortcut, then edit its key combination and action order.",
    )?;
    let list = create_list(hwnd, SHORTCUT_LIST)?;
    for shortcut in &settings.shortcuts {
        send_text(list, LB_ADDSTRING, &shortcut_label(shortcut, settings));
    }
    create_button(hwnd, SHORTCUT_NEW, "New")?;
    create_button(hwnd, SHORTCUT_DELETE, "Delete")?;
    create_checkbox(hwnd, SHORTCUT_CONTROL, "Control", false)?;
    create_checkbox(hwnd, SHORTCUT_ALT, "Alt", false)?;
    create_checkbox(hwnd, SHORTCUT_SHIFT, "Shift", false)?;
    create_checkbox(hwnd, SHORTCUT_WIN, "Windows", false)?;
    create_text(hwnd, SHORTCUT_KEY + 1000, "Shortcut key")?;
    create_combo(hwnd, SHORTCUT_KEY)?;
    create_text(
        hwnd,
        1732,
        "Actions run in the order shown. Add at least one action to each shortcut.",
    )?;
    create_list(hwnd, SHORTCUT_CYCLE_LIST)?;
    create_text(hwnd, SHORTCUT_ACTION_PICKER + 1000, "Action")?;
    let picker = create_combo(hwnd, SHORTCUT_ACTION_PICKER)?;
    for action in action_options(settings) {
        send_text(picker, CB_ADDSTRING, &action_label(action, settings));
    }
    create_button(hwnd, SHORTCUT_ACTION_ADD, "Add action")?;
    create_button(hwnd, SHORTCUT_ACTION_REMOVE, "Remove action")?;
    create_button(hwnd, SHORTCUT_APPLY, "Add shortcut")?;
    create_text(hwnd, SHORTCUTS_ERROR, "")?;
    let selected = if settings.shortcuts.is_empty() {
        None
    } else {
        Some(0)
    };
    load_shortcut_editor(hwnd, selected);
    Ok(())
}

fn build_custom_frames(hwnd: HWND, settings: &Settings) -> Result<(), String> {
    create_group(hwnd, 1740, "Custom monitor layouts")?;
    create_text(
        hwnd,
        1741,
        "Coordinates are fractions of the monitor work area from 0 to 1.",
    )?;
    let list = create_list(hwnd, FRAME_LIST)?;
    for frame in &settings.custom_frames {
        send_text(list, LB_ADDSTRING, &frame_label(frame));
    }
    create_button(hwnd, FRAME_NEW, "New")?;
    create_button(hwnd, FRAME_DELETE, "Delete")?;
    create_text(hwnd, FRAME_NAME + 1000, "Name")?;
    create_edit(hwnd, FRAME_NAME, "", false)?;
    create_text(hwnd, field_error_id(FRAME_NAME), "")?;
    create_decimal(hwnd, FRAME_X, "X", 0.0)?;
    create_decimal(hwnd, FRAME_Y, "Y", 0.0)?;
    create_decimal(hwnd, FRAME_WIDTH, "Width", 1.0)?;
    create_decimal(hwnd, FRAME_HEIGHT, "Height", 1.0)?;
    create_button(hwnd, FRAME_APPLY, "Add frame")?;
    create_text(hwnd, FRAMES_ERROR, "")?;
    let selected = if settings.custom_frames.is_empty() {
        None
    } else {
        Some(0)
    };
    load_frame_editor(hwnd, selected);
    Ok(())
}

fn load_shortcut_editor(hwnd: HWND, selected: Option<usize>) {
    let settings = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|state| state.settings.clone())
            .unwrap_or_default()
    });
    let shortcut = selected.and_then(|index| settings.shortcuts.get(index).cloned());
    let hotkey = shortcut.as_ref().map(|item| item.hotkey).unwrap_or(Hotkey {
        control: true,
        alt: true,
        shift: false,
        win: false,
        key: u16::from(b'A'),
    });
    let actions = shortcut
        .as_ref()
        .map(|item| item.actions.clone())
        .unwrap_or_default();
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.editing_shortcut = shortcut.as_ref().and(selected);
            state.shortcut_actions = actions.clone();
        }
    });
    if let Some(index) = selected {
        if let Ok(list) = unsafe { GetDlgItem(Some(hwnd), SHORTCUT_LIST) } {
            unsafe {
                SendMessageW(list, LB_SETCURSEL, Some(WPARAM(index)), None);
            }
        }
    } else if let Ok(list) = unsafe { GetDlgItem(Some(hwnd), SHORTCUT_LIST) } {
        unsafe {
            SendMessageW(list, LB_SETCURSEL, Some(WPARAM(usize::MAX)), None);
        }
    }
    set_checked_by_id(hwnd, SHORTCUT_CONTROL, hotkey.control);
    set_checked_by_id(hwnd, SHORTCUT_ALT, hotkey.alt);
    set_checked_by_id(hwnd, SHORTCUT_SHIFT, hotkey.shift);
    set_checked_by_id(hwnd, SHORTCUT_WIN, hotkey.win);
    if let Ok(combo) = unsafe { GetDlgItem(Some(hwnd), SHORTCUT_KEY) } {
        fill_key_combo(combo, hotkey.key);
    }
    refresh_shortcut_cycle_list(hwnd);
    set_control_text(
        hwnd,
        SHORTCUT_APPLY,
        if selected.is_some() {
            "Update shortcut"
        } else {
            "Add shortcut"
        },
    );
    set_control_text(hwnd, SHORTCUTS_ERROR, "");
}

fn refresh_shortcut_list(hwnd: HWND, selected: Option<usize>) {
    let settings = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|state| state.settings.clone())
            .unwrap_or_default()
    });
    if let Ok(list) = unsafe { GetDlgItem(Some(hwnd), SHORTCUT_LIST) } {
        unsafe {
            SendMessageW(
                list,
                windows::Win32::UI::WindowsAndMessaging::LB_RESETCONTENT,
                None,
                None,
            );
        }
        for shortcut in &settings.shortcuts {
            send_text(list, LB_ADDSTRING, &shortcut_label(shortcut, &settings));
        }
        if let Some(index) = selected.filter(|index| *index < settings.shortcuts.len()) {
            unsafe {
                SendMessageW(list, LB_SETCURSEL, Some(WPARAM(index)), None);
            }
        }
    }
}

fn refresh_shortcut_cycle_list(hwnd: HWND) {
    let settings = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|state| state.settings.clone())
            .unwrap_or_default()
    });
    let actions = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|state| state.shortcut_actions.clone())
            .unwrap_or_default()
    });
    if let Ok(list) = unsafe { GetDlgItem(Some(hwnd), SHORTCUT_CYCLE_LIST) } {
        unsafe {
            SendMessageW(
                list,
                windows::Win32::UI::WindowsAndMessaging::LB_RESETCONTENT,
                None,
                None,
            );
        }
        for (index, action) in actions.iter().enumerate() {
            send_text(
                list,
                LB_ADDSTRING,
                &format!("{}. {}", index + 1, action_label(*action, &settings)),
            );
        }
        if !actions.is_empty() {
            unsafe {
                SendMessageW(list, LB_SETCURSEL, Some(WPARAM(0)), None);
            }
        }
    }
}

fn select_shortcut(hwnd: HWND) {
    let selected = selected_list_item(hwnd, SHORTCUT_LIST);
    load_shortcut_editor(hwnd, selected);
}

fn new_shortcut(hwnd: HWND) {
    load_shortcut_editor(hwnd, None);
    set_status(
        hwnd,
        "Enter a key combination and add actions to create a shortcut.",
    );
}

fn delete_shortcut(hwnd: HWND) {
    let Some(index) = selected_list_item(hwnd, SHORTCUT_LIST) else {
        return;
    };
    let (next_index, changed) = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else {
            return (None, false);
        };
        if index >= state.settings.shortcuts.len() {
            return (None, false);
        }
        state.settings.shortcuts.remove(index);
        let next = (!state.settings.shortcuts.is_empty())
            .then(|| index.min(state.settings.shortcuts.len() - 1));
        (next, true)
    });
    if changed {
        refresh_shortcut_list(hwnd, next_index);
        load_shortcut_editor(hwnd, next_index);
        set_status(
            hwnd,
            "Shortcut removed from the editor. Save to apply the change.",
        );
    }
}

fn add_shortcut_action(hwnd: HWND) {
    let Some(index) = combo_index(hwnd, SHORTCUT_ACTION_PICKER) else {
        set_status(hwnd, "Choose an action first.");
        return;
    };
    let settings = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|state| state.settings.clone())
            .unwrap_or_default()
    });
    let Some(action) = action_options(&settings).get(index).copied() else {
        set_status(hwnd, "The selected action is unavailable.");
        return;
    };
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.shortcut_actions.push(action);
        }
    });
    refresh_shortcut_cycle_list(hwnd);
    set_control_text(hwnd, SHORTCUTS_ERROR, "");
}

fn remove_shortcut_action(hwnd: HWND) {
    let Some(index) = selected_list_item(hwnd, SHORTCUT_CYCLE_LIST) else {
        return;
    };
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut()
            && index < state.shortcut_actions.len()
        {
            state.shortcut_actions.remove(index);
        }
    });
    refresh_shortcut_cycle_list(hwnd);
}

fn apply_shortcut_editor(hwnd: HWND) {
    let mut settings = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|state| state.settings.clone())
            .unwrap_or_default()
    });
    match write_shortcut_editor(hwnd, &mut settings, false) {
        Ok(index) => {
            STATE.with(|state| {
                if let Some(state) = state.borrow_mut().as_mut() {
                    state.settings = settings;
                    state.status =
                        "Shortcut updated in the editor. Save to apply the change.".into();
                }
            });
            refresh_shortcut_list(hwnd, Some(index));
            load_shortcut_editor(hwnd, Some(index));
            refresh_status(hwnd);
        }
        Err(error) => show_form_error(hwnd, SHORTCUT_LIST, &error),
    }
}

fn write_shortcut_editor(
    hwnd: HWND,
    settings: &mut Settings,
    allow_empty_new: bool,
) -> Result<usize, String> {
    let hotkey =
        read_shortcut_hotkey(hwnd).ok_or_else(|| "Choose a key for this shortcut.".to_string())?;
    if hotkey == settings.trigger {
        return Err("A shortcut cannot use the same key combination as the main trigger.".into());
    }
    let editing = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .and_then(|state| state.editing_shortcut)
    });
    let actions = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|state| state.shortcut_actions.clone())
            .unwrap_or_default()
    });
    if actions.is_empty() {
        if allow_empty_new && editing.is_none() {
            return Ok(settings.shortcuts.len());
        }
        return Err("Add at least one action to the shortcut cycle.".into());
    }
    if settings
        .shortcuts
        .iter()
        .enumerate()
        .any(|(index, shortcut)| Some(index) != editing && shortcut.hotkey == hotkey)
    {
        return Err("Each shortcut must use a different key combination.".into());
    }
    let value = Shortcut { hotkey, actions };
    if let Some(index) = editing {
        if index >= settings.shortcuts.len() {
            return Err("The selected shortcut no longer exists.".into());
        }
        settings.shortcuts[index] = value;
        Ok(index)
    } else {
        settings.shortcuts.push(value);
        Ok(settings.shortcuts.len() - 1)
    }
}

fn read_shortcut_hotkey(hwnd: HWND) -> Option<Hotkey> {
    let hotkey = Hotkey {
        control: is_checked(hwnd, SHORTCUT_CONTROL),
        alt: is_checked(hwnd, SHORTCUT_ALT),
        shift: is_checked(hwnd, SHORTCUT_SHIFT),
        win: is_checked(hwnd, SHORTCUT_WIN),
        key: selected_key(hwnd, SHORTCUT_KEY)?,
    };
    (hotkey.control || hotkey.alt || hotkey.shift || hotkey.win).then_some(hotkey)
}

fn shortcut_label(shortcut: &Shortcut, settings: &Settings) -> String {
    let actions = shortcut
        .actions
        .iter()
        .map(|action| action_label(*action, settings))
        .collect::<Vec<_>>()
        .join(" → ");
    format!("{}   →   {}", format_hotkey(shortcut.hotkey), actions)
}

fn selected_list_item(hwnd: HWND, id: i32) -> Option<usize> {
    let list = unsafe { GetDlgItem(Some(hwnd), id) }.ok()?;
    let selected = unsafe { SendMessageW(list, LB_GETCURSEL, None, None).0 };
    usize::try_from(selected).ok()
}

fn set_checked_by_id(hwnd: HWND, id: i32, value: bool) {
    if let Ok(control) = unsafe { GetDlgItem(Some(hwnd), id) } {
        set_checked(control, value);
    }
}

fn set_enabled_by_id(hwnd: HWND, id: i32, enabled: bool) {
    if let Ok(control) = unsafe { GetDlgItem(Some(hwnd), id) } {
        unsafe {
            let _ = EnableWindow(control, enabled);
        }
    }
}

fn update_radial_color_enabled(hwnd: HWND) {
    let enabled = !is_checked(hwnd, RADIAL_SYSTEM_ACCENT);
    for id in [
        RADIAL_COLOR + 1000,
        RADIAL_COLOR,
        field_error_id(RADIAL_COLOR),
    ] {
        set_enabled_by_id(hwnd, id, enabled);
    }
}

fn update_gradient_enabled(hwnd: HWND) {
    let enabled = is_checked(hwnd, RADIAL_GRADIENT);
    for id in [
        RADIAL_GRADIENT_COLOR + 1000,
        RADIAL_GRADIENT_COLOR,
        field_error_id(RADIAL_GRADIENT_COLOR),
    ] {
        set_enabled_by_id(hwnd, id, enabled);
    }
}

fn update_edge_padding_enabled(hwnd: HWND) {
    let enabled = is_checked(hwnd, ADV_EDGE_ENABLED);
    for id in [ADV_EDGE_TOP, ADV_EDGE_RIGHT, ADV_EDGE_BOTTOM, ADV_EDGE_LEFT] {
        for control in [id + 1000, id, field_error_id(id)] {
            set_enabled_by_id(hwnd, control, enabled);
        }
    }
}

fn update_animation_enabled(hwnd: HWND) {
    let enabled = is_checked(hwnd, ADV_ANIMATE_WINDOWS) || is_checked(hwnd, ADV_ANIMATE_STASHED);
    for id in [
        ADV_ANIMATION_DURATION + 1000,
        ADV_ANIMATION_DURATION,
        field_error_id(ADV_ANIMATION_DURATION),
    ] {
        set_enabled_by_id(hwnd, id, enabled);
    }
}

fn update_middle_click_delay_enabled(hwnd: HWND) {
    set_enabled_by_id(hwnd, ADV_MIDDLE_DELAY, is_checked(hwnd, ADV_MIDDLE_CLICK));
}

fn set_combo_selection(control: HWND, index: usize) {
    unsafe {
        SendMessageW(control, CB_SETCURSEL, Some(WPARAM(index)), None);
    }
}

fn load_frame_editor(hwnd: HWND, selected: Option<usize>) {
    let settings = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|state| state.settings.clone())
            .unwrap_or_default()
    });
    let frame = selected.and_then(|index| settings.custom_frames.get(index).cloned());
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.editing_frame = frame.as_ref().and(selected);
        }
    });
    if let Some(index) = selected {
        if let Ok(list) = unsafe { GetDlgItem(Some(hwnd), FRAME_LIST) } {
            unsafe {
                SendMessageW(list, LB_SETCURSEL, Some(WPARAM(index)), None);
            }
        }
    } else if let Ok(list) = unsafe { GetDlgItem(Some(hwnd), FRAME_LIST) } {
        unsafe {
            SendMessageW(list, LB_SETCURSEL, Some(WPARAM(usize::MAX)), None);
        }
    }
    if let Some(frame) = frame {
        set_control_text(hwnd, FRAME_NAME, &frame.name);
        set_control_text(hwnd, FRAME_X, &format_float(frame.x));
        set_control_text(hwnd, FRAME_Y, &format_float(frame.y));
        set_control_text(hwnd, FRAME_WIDTH, &format_float(frame.width));
        set_control_text(hwnd, FRAME_HEIGHT, &format_float(frame.height));
        set_control_text(hwnd, FRAME_APPLY, "Update frame");
    } else {
        set_control_text(hwnd, FRAME_NAME, "");
        set_control_text(hwnd, FRAME_X, "0");
        set_control_text(hwnd, FRAME_Y, "0");
        set_control_text(hwnd, FRAME_WIDTH, "1");
        set_control_text(hwnd, FRAME_HEIGHT, "1");
        set_control_text(hwnd, FRAME_APPLY, "Add frame");
    }
    for id in [FRAME_X, FRAME_Y, FRAME_WIDTH, FRAME_HEIGHT] {
        set_control_text(hwnd, field_error_id(id), "");
    }
    set_control_text(hwnd, FRAMES_ERROR, "");
}

fn refresh_frame_list(hwnd: HWND, selected: Option<usize>) {
    let settings = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|state| state.settings.clone())
            .unwrap_or_default()
    });
    if let Ok(list) = unsafe { GetDlgItem(Some(hwnd), FRAME_LIST) } {
        unsafe {
            SendMessageW(
                list,
                windows::Win32::UI::WindowsAndMessaging::LB_RESETCONTENT,
                None,
                None,
            );
        }
        for frame in &settings.custom_frames {
            send_text(list, LB_ADDSTRING, &frame_label(frame));
        }
        if let Some(index) = selected.filter(|index| *index < settings.custom_frames.len()) {
            unsafe {
                SendMessageW(list, LB_SETCURSEL, Some(WPARAM(index)), None);
            }
        }
    }
}

fn select_frame(hwnd: HWND) {
    load_frame_editor(hwnd, selected_list_item(hwnd, FRAME_LIST));
}

fn new_frame(hwnd: HWND) {
    load_frame_editor(hwnd, None);
    set_status(
        hwnd,
        "Enter a name and work-area fractions to create a custom frame.",
    );
}

fn apply_frame_editor(hwnd: HWND) {
    let mut settings = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|state| state.settings.clone())
            .unwrap_or_default()
    });
    match write_frame_editor(hwnd, &mut settings, true) {
        Ok(index) => {
            STATE.with(|state| {
                if let Some(state) = state.borrow_mut().as_mut() {
                    state.settings = settings;
                    state.status =
                        "Custom frame updated in the editor. Save to apply the change.".into();
                }
            });
            refresh_frame_list(hwnd, Some(index));
            load_frame_editor(hwnd, Some(index));
            refresh_status(hwnd);
        }
        Err((control, error)) => show_form_error(hwnd, control, &error),
    }
}

fn write_frame_editor(
    hwnd: HWND,
    settings: &mut Settings,
    require_name: bool,
) -> Result<usize, (i32, String)> {
    let name = read_control_text(hwnd, FRAME_NAME).trim().to_string();
    let editing = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .and_then(|state| state.editing_frame)
    });
    if name.is_empty() && !require_name && editing.is_none() {
        return Ok(settings.custom_frames.len());
    }
    if name.is_empty() {
        let error = "Enter a name for this custom frame.";
        set_control_text(hwnd, FRAMES_ERROR, error);
        return Err((FRAME_NAME, error.into()));
    }
    let x = read_fraction(hwnd, FRAME_X, "X")?;
    let y = read_fraction(hwnd, FRAME_Y, "Y")?;
    let width = read_fraction(hwnd, FRAME_WIDTH, "Width")?;
    let height = read_fraction(hwnd, FRAME_HEIGHT, "Height")?;
    if width <= 0.0 || height <= 0.0 || x + width > 1.0 || y + height > 1.0 {
        let error = "The frame must fit inside the work area; width and height must be positive, and right/bottom cannot exceed 1.";
        set_control_text(hwnd, FRAMES_ERROR, error);
        return Err((FRAME_WIDTH, error.into()));
    }
    if settings
        .custom_frames
        .iter()
        .enumerate()
        .any(|(index, frame)| Some(index) != editing && frame.name.eq_ignore_ascii_case(&name))
    {
        let error = "Custom frame names must be unique.";
        set_control_text(hwnd, FRAMES_ERROR, error);
        return Err((FRAME_NAME, error.into()));
    }
    let frame = CustomFrame {
        name,
        x,
        y,
        width,
        height,
    };
    if let Some(index) = editing {
        if index >= settings.custom_frames.len() {
            return Err((
                FRAME_LIST,
                "The selected custom frame no longer exists.".into(),
            ));
        }
        settings.custom_frames[index] = frame;
        Ok(index)
    } else {
        settings.custom_frames.push(frame);
        Ok(settings.custom_frames.len() - 1)
    }
}

fn read_fraction(hwnd: HWND, id: i32, label: &str) -> Result<f64, (i32, String)> {
    let text = read_control_text(hwnd, id);
    match text.trim().parse::<f64>() {
        Ok(value) if value.is_finite() && (0.0..=1.0).contains(&value) => {
            set_control_text(hwnd, field_error_id(id), "");
            Ok(value)
        }
        _ => {
            let error = format!("{label} must be a number from 0 to 1.");
            set_field_error(hwnd, id, &error);
            Err((id, error))
        }
    }
}

fn delete_frame(hwnd: HWND) {
    let Some(index) = selected_list_item(hwnd, FRAME_LIST) else {
        return;
    };
    let mut settings = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|state| state.settings.clone())
            .unwrap_or_default()
    });
    if index >= settings.custom_frames.len() {
        return;
    }
    let custom_index = index as u16;
    if settings
        .radial_actions
        .contains(&Action::Custom(custom_index))
        || settings
            .shortcuts
            .iter()
            .any(|shortcut| shortcut.actions.contains(&Action::Custom(custom_index)))
    {
        let error = "This frame is assigned to a radial direction or shortcut. Reassign those actions before deleting it.";
        set_control_text(hwnd, FRAMES_ERROR, error);
        set_status(hwnd, error);
        return;
    }
    settings.custom_frames.remove(index);
    for action in &mut settings.radial_actions {
        remap_custom_after_delete(action, custom_index);
    }
    for shortcut in &mut settings.shortcuts {
        for action in &mut shortcut.actions {
            remap_custom_after_delete(action, custom_index);
        }
    }
    let selected =
        (!settings.custom_frames.is_empty()).then(|| index.min(settings.custom_frames.len() - 1));
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.settings = settings;
            state.status = "Custom frame removed from the editor. Save to apply the change.".into();
        }
    });
    refresh_frame_list(hwnd, selected);
    load_frame_editor(hwnd, selected);
    refresh_status(hwnd);
}

fn remap_custom_after_delete(action: &mut Action, deleted: u16) {
    if let Action::Custom(index) = action
        && *index > deleted
    {
        *index -= 1;
    }
}

fn frame_label(frame: &CustomFrame) -> String {
    format!(
        "{}   ({}, {}, {}, {})",
        frame.name,
        format_float(frame.x),
        format_float(frame.y),
        format_float(frame.width),
        format_float(frame.height)
    )
}

fn format_float(value: f64) -> String {
    format!("{value:.4}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn build_exclusions(hwnd: HWND, settings: &Settings) -> Result<(), String> {
    create_group(hwnd, 1750, "Excluded applications")?;
    create_text(
        hwnd,
        1751,
        "Enter one executable name per line, for example: game.exe",
    )?;
    create_text(
        hwnd,
        1752,
        "Excluded windows will not be moved or resized by Orbit.",
    )?;
    create_edit(
        hwnd,
        EXCLUSIONS_TEXT,
        &settings.excluded_processes.join("\r\n"),
        true,
    )?;
    Ok(())
}

fn build_about(hwnd: HWND) -> Result<(), String> {
    create_group(hwnd, 1760, "Orbit")?;
    create_text(
        hwnd,
        1761,
        "Orbit is a native Windows window manager inspired by Loop.",
    )?;
    create_text(
        hwnd,
        1762,
        "Orbit is licensed under GPL-3.0-only. Copyright holders and source attribution are in the included LICENSE and NOTICE files.",
    )?;
    create_button(hwnd, ABOUT_UPDATE, "Check for updates")?;
    create_text(
        hwnd,
        ABOUT_STATUS,
        "Manual update checks run in the resident Orbit process.",
    )?;
    create_text(
        hwnd,
        1763,
        "Settings are stored in your local application data folder. Use Import and Export below to move a settings file.",
    )?;
    Ok(())
}

fn layout_window(hwnd: HWND) {
    let (width, height) = client_size_logical(hwnd);
    // Keep the list above Reset. A full-height list paints its border through that button.
    move_control(hwnd, SIDEBAR, 18, 18, 186, (height - 84).max(120));
    raise_shell(hwnd);
    move_control(hwnd, PAGE_TITLE, 232, 18, width - 256, 34);
    move_control(hwnd, PAGE_DESCRIPTION, 232, 54, width - 256, 42);
    move_control(hwnd, STATUS, 232, height - 91, width - 256, 24);
    move_control(hwnd, SAVE, width - 106, height - 54, 88, 30);
    move_control(hwnd, CANCEL, width - 202, height - 54, 88, 30);
    move_control(hwnd, EXPORT, width - 298, height - 54, 88, 30);
    move_control(hwnd, IMPORT, width - 394, height - 54, 88, 30);
    move_control(hwnd, RESET, 18, height - 54, 112, 30);
}

fn layout_page(hwnd: HWND, page: usize) {
    let (width, height) = client_size_logical(hwnd);
    update_scrollbar(hwnd, page, height);
    let x = 232;
    let body_y = 108;
    let body_width = (width - 256).max(360);
    let gap = 20;
    match page {
        0 => {
            let column = (body_width - gap) / 2;
            move_control(hwnd, 1700, x, body_y, column, (height - 220).max(1));
            move_control(
                hwnd,
                1701,
                x + column + gap,
                body_y,
                column,
                (height - 220).max(1),
            );
            let row = |n: i32| body_y + 36 + n * 48;
            for (id, n) in [
                (BEHAVIOR_PADDING, 0),
                (BEHAVIOR_SIZE_INCREMENT, 1),
                (BEHAVIOR_SNAP_THRESHOLD, 2),
                (BEHAVIOR_STASH_PADDING, 3),
            ] {
                layout_numeric(hwnd, id, x + 16, row(n), column - 32);
            }
            for (id, n) in [
                (BEHAVIOR_SNAP, 0),
                (BEHAVIOR_SCREEN_CURSOR, 1),
                (BEHAVIOR_RESIZE_CURSOR, 2),
                (BEHAVIOR_FOCUS_RESIZE, 3),
                (BEHAVIOR_MOVE_CURSOR, 4),
                (BEHAVIOR_IGNORE_FULLSCREEN, 5),
                (BEHAVIOR_DISABLE_CURSOR, 6),
                (BEHAVIOR_LOCK_CENTER, 7),
            ] {
                move_control(hwnd, id, x + 16, body_y + 232 + n * 30, column - 32, 26);
            }

            let rx = x + column + gap + 16;
            let modifier_y = body_y + 36;
            for (id, n) in [
                (BEHAVIOR_TRIGGER_CONTROL, 0),
                (BEHAVIOR_TRIGGER_ALT, 1),
                (BEHAVIOR_TRIGGER_SHIFT, 2),
                (BEHAVIOR_TRIGGER_WIN, 3),
            ] {
                move_control(
                    hwnd,
                    id,
                    rx + n * (column - 32) / 4,
                    modifier_y,
                    (column - 32) / 4,
                    26,
                );
            }
            move_control(
                hwnd,
                BEHAVIOR_TRIGGER_KEY + 1000,
                rx,
                body_y + 72,
                column - 120,
                24,
            );
            move_control(
                hwnd,
                BEHAVIOR_TRIGGER_KEY,
                rx + column - 116,
                body_y + 69,
                100,
                240,
            );
            move_control(
                hwnd,
                BEHAVIOR_TRIGGER_ERROR,
                rx,
                body_y + 96,
                column - 32,
                20,
            );
            layout_numeric(hwnd, BEHAVIOR_TRIGGER_DELAY, rx, body_y + 126, column - 32);
            layout_numeric(hwnd, BEHAVIOR_CYCLE_TIMEOUT, rx, body_y + 174, column - 32);
            for (id, n) in [
                (BEHAVIOR_CYCLE_SHIFT, 0),
                (BEHAVIOR_REVERSE_SCROLL, 1),
                (BEHAVIOR_LAUNCH_LOGIN, 2),
                (BEHAVIOR_UPDATES_ENABLED, 3),
            ] {
                move_control(hwnd, id, rx, body_y + 222 + n * 30, column - 28, 26);
            }
        }
        1 => {
            let half = (body_width - gap) / 2;
            move_control(hwnd, 1710, x, body_y, body_width, (height - 220).max(1));
            move_control(
                hwnd,
                RADIAL_VISIBLE,
                x + 16,
                body_y + 34,
                body_width - 32,
                26,
            );
            for (id, y) in [
                (RADIAL_SIZE, 76),
                (RADIAL_THICKNESS, 124),
                (RADIAL_CORNER, 172),
            ] {
                layout_numeric(hwnd, id, x + 16, body_y + y, half - 24);
            }
            move_control(
                hwnd,
                RADIAL_COLOR + 1000,
                x + half + 4,
                body_y + 76,
                half - 18,
                24,
            );
            move_control(
                hwnd,
                RADIAL_COLOR,
                x + half + 4,
                body_y + 102,
                half - 18,
                25,
            );
            move_control(
                hwnd,
                field_error_id(RADIAL_COLOR),
                x + half + 4,
                body_y + 128,
                half - 18,
                18,
            );
            move_control(
                hwnd,
                RADIAL_SYSTEM_ACCENT,
                x + half + 4,
                body_y + 152,
                half - 18,
                26,
            );
            move_control(
                hwnd,
                RADIAL_GRADIENT,
                x + half + 4,
                body_y + 182,
                half - 18,
                26,
            );
            move_control(
                hwnd,
                RADIAL_GRADIENT_COLOR + 1000,
                x + half + 4,
                body_y + 210,
                half - 18,
                22,
            );
            move_control(
                hwnd,
                RADIAL_GRADIENT_COLOR,
                x + half + 4,
                body_y + 234,
                half - 18,
                24,
            );
            move_control(
                hwnd,
                field_error_id(RADIAL_GRADIENT_COLOR),
                x + half + 4,
                body_y + 258,
                half - 18,
                18,
            );
            for slot in 0..ACTION_COUNT {
                let col = slot / 4;
                let row = slot % 4;
                let cx = x + col as i32 * (half + gap) + 16;
                let cy = body_y + 292 + row as i32 * 36;
                move_control(hwnd, ACTION_LABEL_BASE + slot as i32, cx, cy, 112, 22);
                move_control(
                    hwnd,
                    RADIAL_ACTION_BASE + slot as i32,
                    cx + 114,
                    cy - 3,
                    half - 142,
                    250,
                );
            }
            move_control(
                hwnd,
                RADIAL_ERROR,
                x + 16,
                body_y + 426,
                body_width - 32,
                24,
            );
        }
        2 => {
            move_control(hwnd, 1720, x, body_y, body_width, (height - 220).max(1));
            move_control(
                hwnd,
                PREVIEW_VISIBLE,
                x + 16,
                body_y + 38,
                body_width - 32,
                26,
            );
            for (id, y) in [
                (PREVIEW_OPACITY, 92),
                (PREVIEW_PADDING, 140),
                (PREVIEW_CORNER, 188),
                (PREVIEW_BORDER, 236),
            ] {
                layout_numeric(hwnd, id, x + 16, body_y + y, 370);
            }
            move_control(
                hwnd,
                PREVIEW_WINDOW_CORNERS,
                x + 16,
                body_y + 284,
                body_width - 32,
                26,
            );
            move_control(hwnd, 1721, x + 16, body_y + 318, body_width - 32, 44);
        }
        3 => {
            move_control(hwnd, 1730, x, body_y, body_width, (height - 220).max(1));
            move_control(hwnd, 1731, x + 16, body_y + 34, body_width - 32, 24);
            let detail_x = x + 304;
            let detail_width = body_width - 320;
            move_control(hwnd, SHORTCUT_LIST, x + 16, body_y + 70, 264, 300);
            move_control(hwnd, SHORTCUT_NEW, x + 16, body_y + 376, 104, 28);
            move_control(hwnd, SHORTCUT_DELETE, x + 132, body_y + 376, 104, 28);
            for (id, index) in [
                (SHORTCUT_CONTROL, 0),
                (SHORTCUT_ALT, 1),
                (SHORTCUT_SHIFT, 2),
                (SHORTCUT_WIN, 3),
            ] {
                move_control(
                    hwnd,
                    id,
                    detail_x + index * detail_width / 4,
                    body_y + 70,
                    detail_width / 4,
                    26,
                );
            }
            move_control(
                hwnd,
                SHORTCUT_KEY + 1000,
                detail_x,
                body_y + 108,
                detail_width - 114,
                22,
            );
            move_control(
                hwnd,
                SHORTCUT_KEY,
                detail_x + detail_width - 108,
                body_y + 104,
                100,
                230,
            );
            move_control(hwnd, 1732, detail_x, body_y + 142, detail_width, 22);
            move_control(
                hwnd,
                SHORTCUT_CYCLE_LIST,
                detail_x,
                body_y + 168,
                detail_width,
                100,
            );
            move_control(
                hwnd,
                SHORTCUT_ACTION_PICKER + 1000,
                detail_x,
                body_y + 278,
                detail_width - 196,
                22,
            );
            move_control(
                hwnd,
                SHORTCUT_ACTION_PICKER,
                detail_x,
                body_y + 304,
                detail_width - 196,
                190,
            );
            move_control(
                hwnd,
                SHORTCUT_ACTION_ADD,
                detail_x + detail_width - 184,
                body_y + 302,
                176,
                28,
            );
            move_control(
                hwnd,
                SHORTCUT_ACTION_REMOVE,
                detail_x + detail_width - 184,
                body_y + 336,
                176,
                28,
            );
            move_control(
                hwnd,
                SHORTCUTS_ERROR,
                detail_x,
                body_y + 374,
                detail_width,
                30,
            );
            move_control(hwnd, SHORTCUT_APPLY, detail_x, body_y + 402, 148, 28);
        }
        4 => {
            move_control(hwnd, 1740, x, body_y, body_width, (height - 220).max(1));
            move_control(hwnd, 1741, x + 16, body_y + 34, body_width - 32, 24);
            move_control(hwnd, FRAME_LIST, x + 16, body_y + 70, 264, 300);
            move_control(hwnd, FRAME_NEW, x + 16, body_y + 376, 104, 28);
            move_control(hwnd, FRAME_DELETE, x + 132, body_y + 376, 104, 28);
            let form_x = x + 304;
            let form_width = body_width - 320;
            move_control(
                hwnd,
                FRAME_NAME + 1000,
                form_x,
                body_y + 70,
                form_width - 8,
                22,
            );
            move_control(hwnd, FRAME_NAME, form_x, body_y + 94, form_width - 8, 24);
            move_control(
                hwnd,
                field_error_id(FRAME_NAME),
                form_x,
                body_y + 119,
                form_width - 8,
                16,
            );
            let half = (form_width - 16) / 2;
            layout_numeric(hwnd, FRAME_X, form_x, body_y + 140, half);
            layout_numeric(hwnd, FRAME_Y, form_x + half + 16, body_y + 140, half);
            layout_numeric(hwnd, FRAME_WIDTH, form_x, body_y + 190, half);
            layout_numeric(hwnd, FRAME_HEIGHT, form_x + half + 16, body_y + 190, half);
            move_control(hwnd, FRAMES_ERROR, form_x, body_y + 242, form_width, 42);
            move_control(hwnd, FRAME_APPLY, form_x, body_y + 294, 148, 30);
        }
        5 => {
            move_control(hwnd, 1750, x, body_y, body_width, (height - 220).max(1));
            move_control(hwnd, 1751, x + 16, body_y + 34, body_width - 32, 24);
            move_control(hwnd, 1752, x + 16, body_y + 60, body_width - 32, 24);
            move_control(
                hwnd,
                EXCLUSIONS_TEXT,
                x + 16,
                body_y + 96,
                body_width - 32,
                (height - body_y - 238).max(200),
            );
        }
        6 => {
            let column = (body_width - gap) / 2;
            let right_x = x + column + gap;
            move_control(
                hwnd,
                ADV_GROUP_PLACEMENT,
                x,
                body_y,
                column,
                (height - 220).max(1),
            );
            move_control(
                hwnd,
                ADV_GROUP_INPUT,
                right_x,
                body_y,
                column,
                (height - 220).max(1),
            );
            layout_numeric(
                hwnd,
                ADV_MIN_SCREEN_INCHES,
                x + 16,
                body_y + 38,
                column - 32,
            );
            move_control(hwnd, ADV_EDGE_ENABLED, x + 16, body_y + 82, column - 32, 26);
            for (id, y) in [
                (ADV_EDGE_TOP, 116),
                (ADV_EDGE_RIGHT, 164),
                (ADV_EDGE_BOTTOM, 212),
                (ADV_EDGE_LEFT, 260),
            ] {
                layout_numeric(hwnd, id, x + 16, body_y + y, column - 32);
            }
            for (id, y) in [
                (ADV_RESTORE_ON_DRAG, 310),
                (ADV_SHIFT_FOCUS_STASHED, 340),
                (ADV_ANIMATE_WINDOWS, 370),
                (ADV_ANIMATE_STASHED, 400),
            ] {
                move_control(hwnd, id, x + 16, body_y + y, column - 28, 26);
            }
            layout_numeric(
                hwnd,
                ADV_ANIMATION_DURATION,
                x + 16,
                body_y + 432,
                column - 32,
            );
            move_control(
                hwnd,
                ADV_IGNORE_LOW_POWER,
                x + 16,
                body_y + 480,
                column - 28,
                26,
            );

            move_control(
                hwnd,
                ADV_PREVIEW_START + 1000,
                right_x + 16,
                body_y + 38,
                column - 32,
                22,
            );
            move_control(
                hwnd,
                ADV_PREVIEW_START,
                right_x + 16,
                body_y + 62,
                column - 32,
                96,
            );
            move_control(
                hwnd,
                ADV_TRIGGER_SIDE + 1000,
                right_x + 16,
                body_y + 166,
                column - 32,
                22,
            );
            move_control(
                hwnd,
                ADV_TRIGGER_SIDE,
                right_x + 16,
                body_y + 190,
                column - 32,
                96,
            );
            for (id, y) in [
                (ADV_CYCLE_RESTART, 292),
                (ADV_DOUBLE_TAP, 322),
                (ADV_MIDDLE_CLICK, 352),
                (ADV_MIDDLE_DELAY, 382),
            ] {
                move_control(hwnd, id, right_x + 16, body_y + y, column - 28, 26);
            }
            layout_numeric(
                hwnd,
                ADV_TRIGGER_TIMEOUT,
                right_x + 16,
                body_y + 412,
                column - 32,
            );
            for (id, y) in [
                (ADV_HIDE_NO_SELECTION, 460),
                (ADV_HIDE_TRAY, 490),
                (ADV_DEV_RELEASES, 520),
            ] {
                move_control(hwnd, id, right_x + 16, body_y + y, column - 28, 26);
            }
        }
        7 => {
            move_control(hwnd, 1760, x, body_y, body_width, (height - 220).max(1));
            move_control(hwnd, 1761, x + 16, body_y + 38, body_width - 32, 24);
            move_control(hwnd, 1762, x + 16, body_y + 72, body_width - 32, 42);
            move_control(hwnd, ABOUT_UPDATE, x + 16, body_y + 138, 148, 30);
            move_control(
                hwnd,
                ABOUT_STATUS,
                x + 178,
                body_y + 140,
                body_width - 198,
                30,
            );
            move_control(hwnd, 1763, x + 16, body_y + 190, body_width - 32, 48);
        }
        _ => {}
    }
    // Page controls are created after the footer. Raise the shell so group frames and
    // combo drop-down rectangles cannot cover footer borders.
    raise_shell(hwnd);
    // Clipped child regions move during scrolling. Erase the uncovered parent areas after all
    // controls have been positioned so stale control backgrounds cannot remain as gray bands.
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, true);
    }
}

fn layout_numeric(hwnd: HWND, id: i32, x: i32, y: i32, width: i32) {
    move_control(hwnd, id + 1000, x, y, width - 94, 20);
    move_control(hwnd, id, x + width - 88, y - 2, 78, 24);
    move_control(hwnd, field_error_id(id), x, y + 25, width, 16);
}

fn update_caption(hwnd: HWND, page: usize) {
    let version = env!("CARGO_PKG_VERSION");
    let descriptions = [
        "Set how Orbit places windows, starts, and responds to the trigger.",
        "Choose the ring shape, accent, and action assigned to each direction.",
        "Tune the live window preview shown while selecting a layout.",
        "Create keyboard shortcuts that cycle through window actions.",
        "Define reusable layouts using fractions of each monitor work area.",
        "Keep selected applications out of Orbit window management.",
        "Fine-tune edge padding, animations, trigger input, and cycling.",
        "Orbit version and update controls.",
    ];
    if let Some(title) = PAGE_NAMES.get(page) {
        set_control_text(hwnd, PAGE_TITLE, title);
    }
    if page == 7 {
        set_control_text(
            hwnd,
            PAGE_DESCRIPTION,
            &format!("Orbit version {version} and update controls."),
        );
    } else if let Some(description) = descriptions.get(page) {
        set_control_text(
            hwnd,
            PAGE_DESCRIPTION,
            &format!("{description}  Scroll with the wheel or Ctrl+Page Up/Down."),
        );
    }
}

fn switch_page(hwnd: HWND) {
    let requested = unsafe { GetDlgItem(Some(hwnd), SIDEBAR) }
        .map(|sidebar| unsafe { SendMessageW(sidebar, LB_GETCURSEL, None, None).0 as usize })
        .unwrap_or(0);
    let current = STATE.with(|state| state.borrow().as_ref().map_or(0, |state| state.page));
    if requested == current || requested >= PAGE_NAMES.len() {
        return;
    }
    if let Err((control, error)) = capture_current_page(hwnd) {
        show_form_error(hwnd, control, &error);
        unsafe {
            if let Ok(sidebar) = GetDlgItem(Some(hwnd), SIDEBAR) {
                SendMessageW(sidebar, LB_SETCURSEL, Some(WPARAM(current)), None);
            }
        }
        return;
    }
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.page = requested;
            state.scroll_offset = 0;
        }
    });
    clear_page_error(hwnd, current);
    if let Err(error) = build_page(hwnd, requested) {
        set_status(hwnd, &error);
    }
    refresh_status(hwnd);
}

fn capture_current_page(hwnd: HWND) -> Result<(), (i32, String)> {
    let Some(mut candidate) =
        STATE.with(|state| state.borrow().as_ref().map(|state| state.settings.clone()))
    else {
        return Err((0, "Settings window state is unavailable.".into()));
    };
    let page = STATE.with(|state| state.borrow().as_ref().map_or(0, |state| state.page));
    capture_page(hwnd, page, &mut candidate)?;
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.settings = candidate;
        }
    });
    Ok(())
}

fn capture_page(hwnd: HWND, page: usize, settings: &mut Settings) -> Result<(), (i32, String)> {
    match page {
        0 => {
            settings.padding = read_int(hwnd, BEHAVIOR_PADDING, "Window padding", 0, 100)? as i32;
            settings.size_increment =
                read_int(hwnd, BEHAVIOR_SIZE_INCREMENT, "Resize step", 1, 2000)? as i32;
            settings.snap_threshold =
                read_int(hwnd, BEHAVIOR_SNAP_THRESHOLD, "Snap threshold", 0, 500)? as i32;
            settings.stash_visible_padding =
                read_int(hwnd, BEHAVIOR_STASH_PADDING, "Visible stash edge", 0, 500)? as i32;
            settings.snap_on_drag = is_checked(hwnd, BEHAVIOR_SNAP);
            settings.use_screen_with_cursor = is_checked(hwnd, BEHAVIOR_SCREEN_CURSOR);
            settings.resize_window_under_cursor = is_checked(hwnd, BEHAVIOR_RESIZE_CURSOR);
            settings.focus_window_on_resize = is_checked(hwnd, BEHAVIOR_FOCUS_RESIZE);
            settings.move_cursor_with_window = is_checked(hwnd, BEHAVIOR_MOVE_CURSOR);
            settings.ignore_fullscreen = is_checked(hwnd, BEHAVIOR_IGNORE_FULLSCREEN);
            settings.disable_cursor_interaction = is_checked(hwnd, BEHAVIOR_DISABLE_CURSOR);
            settings.lock_radial_menu_to_center = is_checked(hwnd, BEHAVIOR_LOCK_CENTER);
            settings.trigger.control = is_checked(hwnd, BEHAVIOR_TRIGGER_CONTROL);
            settings.trigger.alt = is_checked(hwnd, BEHAVIOR_TRIGGER_ALT);
            settings.trigger.shift = is_checked(hwnd, BEHAVIOR_TRIGGER_SHIFT);
            settings.trigger.win = is_checked(hwnd, BEHAVIOR_TRIGGER_WIN);
            settings.trigger.key = selected_key(hwnd, BEHAVIOR_TRIGGER_KEY)
                .ok_or((BEHAVIOR_TRIGGER_KEY, "Choose a trigger key.".into()))?;
            settings.trigger_delay_ms =
                read_int(hwnd, BEHAVIOR_TRIGGER_DELAY, "Trigger delay", 0, 1000)? as u32;
            settings.cycle_timeout_ms =
                read_int(hwnd, BEHAVIOR_CYCLE_TIMEOUT, "Cycle timeout", 50, 60_000)? as u32;
            settings.cycle_backwards_on_shift = is_checked(hwnd, BEHAVIOR_CYCLE_SHIFT);
            settings.reverse_scroll = is_checked(hwnd, BEHAVIOR_REVERSE_SCROLL);
            settings.launch_at_login = is_checked(hwnd, BEHAVIOR_LAUNCH_LOGIN);
            settings.updates_enabled = is_checked(hwnd, BEHAVIOR_UPDATES_ENABLED);
        }
        1 => {
            settings.radial_menu_visible = is_checked(hwnd, RADIAL_VISIBLE);
            settings.radial_size = read_int(hwnd, RADIAL_SIZE, "Ring diameter", 1, 512)? as u32;
            settings.radial_thickness = read_int(
                hwnd,
                RADIAL_THICKNESS,
                "Ring thickness",
                1,
                (settings.radial_size / 2) as i64,
            )? as u32;
            settings.radial_corner_radius = read_int(
                hwnd,
                RADIAL_CORNER,
                "Corner radius",
                0,
                (settings.radial_size / 2) as i64,
            )? as u32;
            settings.use_system_accent = is_checked(hwnd, RADIAL_SYSTEM_ACCENT);
            settings.use_gradient = is_checked(hwnd, RADIAL_GRADIENT);
            if !settings.use_system_accent {
                settings.accent_color = read_rgb(hwnd, RADIAL_COLOR, "Accent color")?;
            }
            if settings.use_gradient {
                settings.gradient_color =
                    read_rgb(hwnd, RADIAL_GRADIENT_COLOR, "Gradient end color")?;
            }
            let options = action_options(settings);
            for slot in 0..ACTION_COUNT {
                let index = combo_index(hwnd, RADIAL_ACTION_BASE + slot as i32).ok_or((
                    RADIAL_ACTION_BASE + slot as i32,
                    "Choose an action for every direction.".into(),
                ))?;
                settings.radial_actions[slot] = options.get(index).copied().ok_or((
                    RADIAL_ACTION_BASE + slot as i32,
                    "The selected radial action is unavailable.".into(),
                ))?;
            }
        }
        2 => {
            settings.preview_visible = is_checked(hwnd, PREVIEW_VISIBLE);
            settings.preview_opacity =
                read_int(hwnd, PREVIEW_OPACITY, "Preview opacity", 0, 255)? as u8;
            settings.preview_padding =
                read_int(hwnd, PREVIEW_PADDING, "Preview inset", 0, 200)? as i32;
            settings.preview_corner_radius =
                read_int(hwnd, PREVIEW_CORNER, "Preview corner radius", 0, 200)? as u32;
            settings.preview_border_thickness =
                read_int(hwnd, PREVIEW_BORDER, "Preview border thickness", 0, 32)? as u32;
            settings.preview_use_window_corner_radius = is_checked(hwnd, PREVIEW_WINDOW_CORNERS);
        }
        3 => {
            write_shortcut_editor(hwnd, settings, true)
                .map_err(|error| (SHORTCUTS_ERROR, error))?;
            set_control_text(hwnd, SHORTCUTS_ERROR, "");
        }
        4 => {
            write_frame_editor(hwnd, settings, false)?;
            set_control_text(hwnd, FRAMES_ERROR, "");
        }
        5 => {
            settings.excluded_processes = read_control_text(hwnd, EXCLUSIONS_TEXT)
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect();
        }
        6 => {
            settings.padding_minimum_screen_inches = read_decimal(
                hwnd,
                ADV_MIN_SCREEN_INCHES,
                "Minimum screen diagonal",
                0.0,
                200.0,
            )?;
            if is_checked(hwnd, ADV_EDGE_ENABLED) {
                settings.edge_padding = Some(EdgePadding {
                    top: read_int(hwnd, ADV_EDGE_TOP, "Top edge padding", 0, 200)? as i32,
                    right: read_int(hwnd, ADV_EDGE_RIGHT, "Right edge padding", 0, 200)? as i32,
                    bottom: read_int(hwnd, ADV_EDGE_BOTTOM, "Bottom edge padding", 0, 200)? as i32,
                    left: read_int(hwnd, ADV_EDGE_LEFT, "Left edge padding", 0, 200)? as i32,
                });
            } else {
                settings.edge_padding = None;
            }
            settings.restore_window_frame_on_drag = is_checked(hwnd, ADV_RESTORE_ON_DRAG);
            settings.shift_focus_when_stashed = is_checked(hwnd, ADV_SHIFT_FOCUS_STASHED);
            settings.animate_window_resizes = is_checked(hwnd, ADV_ANIMATE_WINDOWS);
            settings.animate_stashed_windows = is_checked(hwnd, ADV_ANIMATE_STASHED);
            settings.animation_duration_ms =
                read_int(hwnd, ADV_ANIMATION_DURATION, "Animation duration", 0, 2000)? as u32;
            settings.ignore_low_power_mode = is_checked(hwnd, ADV_IGNORE_LOW_POWER);
            settings.preview_start = match combo_index(hwnd, ADV_PREVIEW_START) {
                Some(0) => PreviewStart::ActionCenter,
                Some(1) => PreviewStart::RadialMenu,
                Some(2) => PreviewStart::ScreenCenter,
                _ => return Err((ADV_PREVIEW_START, "Choose where preview starts.".into())),
            };
            settings.trigger_side = match combo_index(hwnd, ADV_TRIGGER_SIDE) {
                Some(0) => TriggerSide::Either,
                Some(1) => TriggerSide::Left,
                Some(2) => TriggerSide::Right,
                _ => return Err((ADV_TRIGGER_SIDE, "Choose a trigger mouse side.".into())),
            };
            settings.cycle_restart = is_checked(hwnd, ADV_CYCLE_RESTART);
            settings.double_tap_to_trigger = is_checked(hwnd, ADV_DOUBLE_TAP);
            settings.middle_click_triggers = is_checked(hwnd, ADV_MIDDLE_CLICK);
            settings.middle_click_uses_delay = is_checked(hwnd, ADV_MIDDLE_DELAY);
            settings.trigger_timeout_ms =
                read_int(hwnd, ADV_TRIGGER_TIMEOUT, "Trigger timeout", 0, 600_000)? as u32;
            settings.hide_on_no_selection = is_checked(hwnd, ADV_HIDE_NO_SELECTION);
            settings.hide_tray_icon = is_checked(hwnd, ADV_HIDE_TRAY);
            settings.include_development_versions = is_checked(hwnd, ADV_DEV_RELEASES);
        }
        7 => {}
        _ => return Err((0, "Unknown settings page.".into())),
    }
    Ok(())
}

fn read_int(hwnd: HWND, id: i32, label: &str, min: i64, max: i64) -> Result<i64, (i32, String)> {
    let text = read_control_text(hwnd, id);
    match text.trim().parse::<i64>() {
        Ok(value) if (min..=max).contains(&value) => {
            set_control_text(hwnd, field_error_id(id), "");
            Ok(value)
        }
        _ => {
            let error = format!("{label} must be a whole number from {min} to {max}.");
            set_field_error(hwnd, id, &error);
            Err((id, error))
        }
    }
}

fn read_decimal(
    hwnd: HWND,
    id: i32,
    label: &str,
    min: f64,
    max: f64,
) -> Result<f64, (i32, String)> {
    let text = read_control_text(hwnd, id);
    match text.trim().parse::<f64>() {
        Ok(value) if value.is_finite() && (min..=max).contains(&value) => {
            set_control_text(hwnd, field_error_id(id), "");
            Ok(value)
        }
        _ => {
            let error = format!("{label} must be a number from {min} to {max}.");
            set_field_error(hwnd, id, &error);
            Err((id, error))
        }
    }
}

fn read_rgb(hwnd: HWND, id: i32, label: &str) -> Result<u32, (i32, String)> {
    let input = read_control_text(hwnd, id);
    let value = input.trim().trim_start_matches('#');
    match (value.len(), u32::from_str_radix(value, 16)) {
        (6, Ok(color)) => {
            set_control_text(hwnd, field_error_id(id), "");
            Ok(color)
        }
        _ => {
            let error = format!("{label} must be six hexadecimal digits, such as #67C1D6.");
            set_field_error(hwnd, id, &error);
            Err((id, error))
        }
    }
}

fn save_settings(hwnd: HWND) {
    if let Err((control, error)) = capture_current_page(hwnd) {
        show_form_error(hwnd, control, &error);
        return;
    }
    let Some(next) =
        STATE.with(|state| state.borrow().as_ref().map(|state| state.settings.clone()))
    else {
        set_status(hwnd, "Settings window state is unavailable.");
        return;
    };
    if let Err(error) = next.validate() {
        show_settings_validation_error(hwnd, current_page(), &error);
        return;
    }
    if let Err(error) = configuration::save(&next) {
        set_status(hwnd, &format!("Could not save and apply settings: {error}"));
        return;
    }
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.settings = next;
            state.status = "Settings saved and applied.".into();
        }
    });
    refresh_status(hwnd);
}

fn reset_editor(hwnd: HWND) {
    let page = current_page();
    if let Err((control, error)) = capture_current_page(hwnd) {
        show_form_error(hwnd, control, &error);
        return;
    }
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.settings = Settings::default();
            state.status = "Defaults loaded in the editor. Save to apply them.".into();
        }
    });
    if let Err(error) = build_page(hwnd, page) {
        set_status(hwnd, &error);
    }
    refresh_status(hwnd);
}

fn import_settings(hwnd: HWND) {
    if let Err((control, error)) = capture_current_page(hwnd) {
        show_form_error(hwnd, control, &error);
        return;
    }
    let Some(path) = (match choose_settings_file(hwnd, false) {
        Ok(path) => path,
        Err(error) => {
            set_status(hwnd, &error);
            return;
        }
    }) else {
        return;
    };
    let settings = match configuration::read_import(&path) {
        Ok(settings) => settings,
        Err(error) => {
            set_status(hwnd, &error);
            return;
        }
    };
    let page = current_page();
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.settings = settings;
            state.status = format!(
                "Imported {} into the editor. Save to apply it.",
                path.display()
            );
        }
    });
    if let Err(error) = build_page(hwnd, page) {
        set_status(hwnd, &error);
    }
    refresh_status(hwnd);
}

fn export_settings(hwnd: HWND) {
    if let Err((control, error)) = capture_current_page(hwnd) {
        show_form_error(hwnd, control, &error);
        return;
    }
    let Some(path) = (match choose_settings_file(hwnd, true) {
        Ok(path) => path,
        Err(error) => {
            set_status(hwnd, &error);
            return;
        }
    }) else {
        return;
    };
    let Some(settings) =
        STATE.with(|state| state.borrow().as_ref().map(|state| state.settings.clone()))
    else {
        set_status(hwnd, "Settings window state is unavailable.");
        return;
    };
    match configuration::export(&settings, &path) {
        Ok(()) => set_status(hwnd, &format!("Settings exported to {}.", path.display())),
        Err(error) => set_status(
            hwnd,
            &format!("{error}. Choose a new file name to export without replacing existing data."),
        ),
    }
}

fn check_for_updates(hwnd: HWND) {
    match platform::check_for_updates() {
        Ok(()) => {
            set_control_text(
                hwnd,
                ABOUT_STATUS,
                "Update check started. Orbit will notify you if an update is available.",
            );
            set_status(hwnd, "Update check started.");
        }
        Err(error) => {
            set_control_text(
                hwnd,
                ABOUT_STATUS,
                &format!("Could not check for updates: {error}"),
            );
            set_status(
                hwnd,
                "Update check could not start. See the About page for details.",
            );
        }
    }
}

fn choose_settings_file(hwnd: HWND, save: bool) -> Result<Option<PathBuf>, String> {
    let filter =
        to_wide("Orbit settings (*.json)\0*.json\0JSON files (*.json)\0*.json\0All files\0*.*\0\0");
    let mut path = vec![0u16; 32_768];
    let title = if save {
        w!("Export Orbit settings")
    } else {
        w!("Import Orbit settings")
    };
    let mut dialog = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrFile: PWSTR(path.as_mut_ptr()),
        nMaxFile: path.len() as u32,
        lpstrTitle: title,
        lpstrDefExt: w!("json"),
        Flags: OFN_NOCHANGEDIR
            | OFN_PATHMUSTEXIST
            | if save {
                Default::default()
            } else {
                OFN_FILEMUSTEXIST
            },
        ..Default::default()
    };
    let chosen = unsafe {
        if save {
            GetSaveFileNameW(&mut dialog).as_bool()
        } else {
            GetOpenFileNameW(&mut dialog).as_bool()
        }
    };
    if chosen {
        let end = path
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(path.len());
        return Ok(Some(PathBuf::from(OsString::from_wide(&path[..end]))));
    }
    let extended_error = unsafe { CommDlgExtendedError() };
    if extended_error.0 == 0 {
        Ok(None)
    } else {
        Err(format!("Windows file dialog failed: {extended_error:?}"))
    }
}

fn format_hotkey(hotkey: Hotkey) -> String {
    let mut parts = Vec::new();
    if hotkey.control {
        parts.push("Ctrl".to_string());
    }
    if hotkey.alt {
        parts.push("Alt".to_string());
    }
    if hotkey.shift {
        parts.push("Shift".to_string());
    }
    if hotkey.win {
        parts.push("Win".to_string());
    }
    parts.push(key_name(hotkey.key));
    parts.join("+")
}

fn key_name(key: u16) -> String {
    match key {
        0x20 => "Space".into(),
        0x0d => "Enter".into(),
        0x09 => "Tab".into(),
        0x1b => "Escape".into(),
        0x08 => "Backspace".into(),
        0x2e => "Delete".into(),
        0x25 => "Left".into(),
        0x26 => "Up".into(),
        0x27 => "Right".into(),
        0x28 => "Down".into(),
        0x70..=0x87 => format!("F{}", key - 0x6f),
        0x30..=0x39 | 0x41..=0x5a => char::from_u32(u32::from(key)).unwrap_or('?').to_string(),
        _ => format!("VK 0x{key:02X}"),
    }
}

fn fill_key_combo(hwnd: HWND, selected: u16) {
    unsafe {
        SendMessageW(
            hwnd,
            windows::Win32::UI::WindowsAndMessaging::CB_RESETCONTENT,
            None,
            None,
        );
    }
    let mut options = key_options();
    if !options.iter().any(|(key, _)| *key == selected) {
        options.push((selected, format!("Virtual key 0x{selected:02X}")));
    }
    let mut selected_index = 0;
    for (index, (key, label)) in options.iter().enumerate() {
        send_text(hwnd, CB_ADDSTRING, label);
        unsafe {
            SendMessageW(
                hwnd,
                CB_SETITEMDATA,
                Some(WPARAM(index)),
                Some(LPARAM(*key as isize)),
            );
        }
        if *key == selected {
            selected_index = index;
        }
    }
    unsafe {
        SendMessageW(hwnd, CB_SETCURSEL, Some(WPARAM(selected_index)), None);
    }
}

fn key_options() -> Vec<(u16, String)> {
    let mut keys = Vec::new();
    for key in b'A'..=b'Z' {
        keys.push((u16::from(key), char::from(key).to_string()));
    }
    for key in b'0'..=b'9' {
        keys.push((u16::from(key), char::from(key).to_string()));
    }
    keys.extend([
        (0x20, "Space".into()),
        (0x0d, "Enter".into()),
        (0x09, "Tab".into()),
        (0x1b, "Escape".into()),
        (0x08, "Backspace".into()),
        (0x2e, "Delete".into()),
        (0x25, "Left arrow".into()),
        (0x26, "Up arrow".into()),
        (0x27, "Right arrow".into()),
        (0x28, "Down arrow".into()),
    ]);
    for key in 1..=24 {
        keys.push((0x6f + key, format!("F{key}")));
    }
    keys
}

fn selected_key(hwnd: HWND, id: i32) -> Option<u16> {
    let control = unsafe { GetDlgItem(Some(hwnd), id) }.ok()?;
    let index = unsafe { SendMessageW(control, CB_GETCURSEL, None, None).0 };
    if index < 0 {
        return None;
    }
    let value =
        unsafe { SendMessageW(control, CB_GETITEMDATA, Some(WPARAM(index as usize)), None).0 };
    u16::try_from(value).ok()
}

fn action_options(settings: &Settings) -> Vec<Action> {
    let mut options = Action::ALL.to_vec();
    let mut custom_indices = std::collections::BTreeSet::new();
    for index in 0..settings.custom_frames.len() {
        if let Ok(index) = u16::try_from(index) {
            custom_indices.insert(index);
        }
    }
    for action in settings.radial_actions.into_iter().chain(
        settings
            .shortcuts
            .iter()
            .flat_map(|shortcut| shortcut.actions.iter().copied()),
    ) {
        if let Action::Custom(index) = action {
            custom_indices.insert(index);
        }
    }
    options.extend(custom_indices.into_iter().map(Action::Custom));
    options
}

fn action_label(action: Action, settings: &Settings) -> String {
    if let Action::Custom(index) = action {
        settings
            .custom_frames
            .get(usize::from(index))
            .map(|frame| format!("Custom: {}", frame.name))
            .unwrap_or_else(|| format!("Custom frame {} (missing)", index))
    } else if action == Action::MacOSCenter {
        "macOS center".into()
    } else {
        action.label().to_string()
    }
}

fn move_control(hwnd: HWND, id: i32, x: i32, y: i32, width: i32, height: i32) {
    if let Ok(control) = unsafe { GetDlgItem(Some(hwnd), id) } {
        let dpi = dpi_for_window(hwnd);
        let page_child = id >= PAGE_ID_START;
        let scrolls_with_page = page_child
            && !matches!(
                id,
                1700 | 1701
                    | 1710
                    | 1720
                    | 1730
                    | 1740
                    | 1750
                    | 1760
                    | ADV_GROUP_PLACEMENT
                    | ADV_GROUP_INPUT
            );
        let offset = if scrolls_with_page {
            STATE.with(|state| {
                state
                    .borrow()
                    .as_ref()
                    .map_or(0, |state| state.scroll_offset)
            })
        } else {
            0
        };
        let y = y - offset;
        let width = width.max(1);
        let height = height.max(1);
        let _ = unsafe {
            SetWindowPos(
                control,
                None,
                px(x, dpi),
                px(y, dpi),
                px(width, dpi),
                px(height, dpi),
                windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER,
            )
        };
        if is_group_id(id) {
            // Group frames sit behind their fields so the frame cannot erase combo text.
            let _ = unsafe {
                SetWindowPos(
                    control,
                    Some(HWND_BOTTOM),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                )
            };
        }
        if page_child {
            let (_, client_height) = client_size_logical(hwnd);
            let clip_bottom = client_height - 112;
            // Leave the group caption band fixed. Scrolling fields start below it.
            let clip_top = if is_group_id(id) { 108 } else { 126 };
            let combo = class_is(control, "ComboBox");
            if combo {
                // A region on a combo cuts the hidden drop-down list into the selection
                // field, which paints a stale copy of the first glyph.
                let selection_visible = y >= clip_top && y + 30 <= clip_bottom;
                unsafe {
                    let _ = SetWindowRgn(control, None, true);
                    let _ = ShowWindow(
                        control,
                        if selection_visible {
                            SW_SHOWNA
                        } else {
                            SW_HIDE
                        },
                    );
                }
            } else {
                let visible_top = y.max(clip_top);
                let visible_bottom = (y + height).min(clip_bottom);
                if visible_bottom <= visible_top {
                    unsafe {
                        let _ = SetWindowRgn(control, None, true);
                        let _ = ShowWindow(control, SW_HIDE);
                    }
                } else {
                    unsafe {
                        let _ = ShowWindow(control, SW_SHOWNA);
                        if visible_top == y && visible_bottom == y + height {
                            let _ = SetWindowRgn(control, None, true);
                        } else {
                            let region = CreateRectRgn(
                                0,
                                px(visible_top - y, dpi),
                                px(width, dpi),
                                px(visible_bottom - y, dpi),
                            );
                            let _ = SetWindowRgn(control, Some(region), true);
                        }
                    }
                }
            }
        }
    }
}

fn is_group_id(id: i32) -> bool {
    matches!(
        id,
        1700 | 1701
            | 1710
            | 1720
            | 1730
            | 1740
            | 1750
            | 1760
            | ADV_GROUP_PLACEMENT
            | ADV_GROUP_INPUT
    )
}

fn raise_shell(hwnd: HWND) {
    for id in [
        SIDEBAR,
        PAGE_TITLE,
        PAGE_DESCRIPTION,
        STATUS,
        IMPORT,
        EXPORT,
        CANCEL,
        SAVE,
        RESET,
    ] {
        if let Ok(control) = unsafe { GetDlgItem(Some(hwnd), id) } {
            let _ = unsafe {
                SetWindowPos(
                    control,
                    Some(HWND_TOP),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                )
            };
        }
    }
}

fn class_is(hwnd: HWND, expected: &str) -> bool {
    let mut name = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut name) };
    if len <= 0 {
        return false;
    }
    let got = String::from_utf16_lossy(&name[..len as usize]);
    got.eq_ignore_ascii_case(expected)
}

fn parent_is_class(control: HWND, expected: &str) -> bool {
    let Ok(parent) = (unsafe { GetParent(control) }) else {
        return false;
    };
    if parent.0.is_null() {
        return false;
    }
    class_is(parent, expected)
}

fn client_size_logical(hwnd: HWND) -> (i32, i32) {
    let mut rect = RECT::default();
    let _ = unsafe { GetClientRect(hwnd, &mut rect) };
    let dpi = dpi_for_window(hwnd);
    (
        ((i64::from(rect.right - rect.left) * 96) / i64::from(dpi)) as i32,
        ((i64::from(rect.bottom - rect.top) * 96) / i64::from(dpi)) as i32,
    )
}

fn dpi_for_window(hwnd: HWND) -> u32 {
    unsafe { GetDpiForWindow(hwnd) }.max(48)
}

fn px(value: i32, dpi: u32) -> i32 {
    ((i64::from(value) * i64::from(dpi) + 48) / 96).clamp(i64::from(i32::MIN), i64::from(i32::MAX))
        as i32
}

fn is_checked(hwnd: HWND, id: i32) -> bool {
    unsafe { GetDlgItem(Some(hwnd), id) }
        .map(|control| unsafe { SendMessageW(control, BM_GETCHECK, None, None).0 == 1 })
        .unwrap_or(false)
}

fn set_checked(control: HWND, checked: bool) {
    unsafe {
        SendMessageW(
            control,
            BM_SETCHECK,
            Some(WPARAM(usize::from(checked))),
            None,
        );
    }
}

fn combo_index(hwnd: HWND, id: i32) -> Option<usize> {
    let control = unsafe { GetDlgItem(Some(hwnd), id) }.ok()?;
    let index = unsafe { SendMessageW(control, CB_GETCURSEL, None, None).0 };
    usize::try_from(index).ok()
}

fn read_control_text(hwnd: HWND, id: i32) -> String {
    let Ok(control) = (unsafe { GetDlgItem(Some(hwnd), id) }) else {
        return String::new();
    };
    let len = unsafe { GetWindowTextLengthW(control) }.max(0) as usize;
    let mut buffer = vec![0u16; len + 1];
    let read = unsafe { GetWindowTextW(control, &mut buffer) }.max(0) as usize;
    String::from_utf16_lossy(&buffer[..read.min(buffer.len())])
}

fn set_control_text(hwnd: HWND, id: i32, text: &str) {
    if let Ok(control) = unsafe { GetDlgItem(Some(hwnd), id) } {
        let wide = to_wide(text);
        let _ = unsafe { SetWindowTextW(control, PCWSTR(wide.as_ptr())) };
    }
}

fn send_text(hwnd: HWND, message: u32, text: &str) {
    let wide = to_wide(text);
    unsafe {
        SendMessageW(hwnd, message, None, Some(LPARAM(wide.as_ptr() as isize)));
    }
}

fn to_wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

fn field_error_id(id: i32) -> i32 {
    FIELD_ERROR_BASE + id - PAGE_ID_START
}

fn current_page() -> usize {
    STATE.with(|state| state.borrow().as_ref().map_or(0, |state| state.page))
}

fn show_settings_validation_error(hwnd: HWND, page: usize, error: &str) {
    let (control, error_id) = match page {
        0 if error.contains("trigger") => (BEHAVIOR_TRIGGER_KEY, BEHAVIOR_TRIGGER_ERROR),
        0 if error.contains("cycle_timeout") => (
            BEHAVIOR_CYCLE_TIMEOUT,
            field_error_id(BEHAVIOR_CYCLE_TIMEOUT),
        ),
        1 if error.contains("gradient_color") => {
            (RADIAL_GRADIENT_COLOR, field_error_id(RADIAL_GRADIENT_COLOR))
        }
        1 if error.contains("color") => (RADIAL_COLOR, field_error_id(RADIAL_COLOR)),
        1 if error.contains("radial_thickness") => {
            (RADIAL_THICKNESS, field_error_id(RADIAL_THICKNESS))
        }
        1 if error.contains("radial_corner_radius") => {
            (RADIAL_CORNER, field_error_id(RADIAL_CORNER))
        }
        1 if error.contains("radial") => (RADIAL_SIZE, field_error_id(RADIAL_SIZE)),
        2 if error.contains("preview_padding") => {
            (PREVIEW_PADDING, field_error_id(PREVIEW_PADDING))
        }
        2 if error.contains("preview_corner_radius") => {
            (PREVIEW_CORNER, field_error_id(PREVIEW_CORNER))
        }
        2 if error.contains("preview_border_thickness") => {
            (PREVIEW_BORDER, field_error_id(PREVIEW_BORDER))
        }
        3 if error.contains("shortcut") || error.contains("custom frame") => {
            (SHORTCUT_LIST, SHORTCUTS_ERROR)
        }
        4 if error.contains("custom frame") => (FRAME_LIST, FRAMES_ERROR),
        6 if error.contains("edge padding") => (ADV_EDGE_TOP, field_error_id(ADV_EDGE_TOP)),
        6 if error.contains("padding_minimum_screen_inches") => {
            (ADV_MIN_SCREEN_INCHES, field_error_id(ADV_MIN_SCREEN_INCHES))
        }
        6 if error.contains("trigger_timeout") => {
            (ADV_TRIGGER_TIMEOUT, field_error_id(ADV_TRIGGER_TIMEOUT))
        }
        6 if error.contains("animation_duration") => (
            ADV_ANIMATION_DURATION,
            field_error_id(ADV_ANIMATION_DURATION),
        ),
        _ => (0, 0),
    };
    if error_id > 0 {
        set_control_text(hwnd, error_id, error);
    }
    if control > 0
        && let Ok(control) = unsafe { GetDlgItem(Some(hwnd), control) }
    {
        unsafe {
            let _ = SetFocus(Some(control));
        }
    }
    set_status(hwnd, error);
}

fn show_form_error(hwnd: HWND, control: i32, error: &str) {
    if control > 0 {
        let error_id = match control {
            SHORTCUTS_ERROR => SHORTCUTS_ERROR,
            SHORTCUT_LIST => SHORTCUTS_ERROR,
            FRAMES_ERROR => FRAMES_ERROR,
            FRAME_LIST => FRAMES_ERROR,
            FRAME_NAME => field_error_id(FRAME_NAME),
            RADIAL_COLOR => field_error_id(RADIAL_COLOR),
            BEHAVIOR_TRIGGER_KEY => BEHAVIOR_TRIGGER_ERROR,
            value
                if (RADIAL_ACTION_BASE..RADIAL_ACTION_BASE + ACTION_COUNT as i32)
                    .contains(&value) =>
            {
                RADIAL_ERROR
            }
            value if (PAGE_ID_START..FIELD_ERROR_BASE).contains(&value) => field_error_id(value),
            _ => 0,
        };
        if error_id > 0 {
            set_control_text(hwnd, error_id, error);
        }
        if let Ok(control) = unsafe { GetDlgItem(Some(hwnd), control) } {
            unsafe {
                let _ = SetFocus(Some(control));
            }
        }
    }
    set_status(hwnd, error);
}

fn clear_page_error(hwnd: HWND, page: usize) {
    let ids = match page {
        0 => vec![BEHAVIOR_TRIGGER_ERROR],
        1 => vec![
            field_error_id(RADIAL_COLOR),
            field_error_id(RADIAL_GRADIENT_COLOR),
            RADIAL_ERROR,
        ],
        3 => vec![SHORTCUTS_ERROR],
        4 => vec![FRAMES_ERROR, field_error_id(FRAME_NAME)],
        6 => vec![
            field_error_id(ADV_MIN_SCREEN_INCHES),
            field_error_id(ADV_EDGE_TOP),
            field_error_id(ADV_EDGE_RIGHT),
            field_error_id(ADV_EDGE_BOTTOM),
            field_error_id(ADV_EDGE_LEFT),
            field_error_id(ADV_ANIMATION_DURATION),
            field_error_id(ADV_TRIGGER_TIMEOUT),
        ],
        _ => Vec::new(),
    };
    for id in ids {
        set_control_text(hwnd, id, "");
    }
    set_status(hwnd, "");
}

fn set_field_error(hwnd: HWND, id: i32, message: &str) {
    set_control_text(hwnd, field_error_id(id), message);
}

fn set_status(hwnd: HWND, message: &str) {
    set_control_text(hwnd, STATUS, message);
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.status = message.to_string();
        }
    });
}

fn refresh_status(hwnd: HWND) {
    let message = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|state| state.status.clone())
            .unwrap_or_default()
    });
    set_control_text(hwnd, STATUS, &message);
}
