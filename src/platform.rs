use crate::history::{self, History};
use crate::settings_window;
use orbit::geometry::{Action, Rect};
use orbit::settings::{Hotkey, Settings};
use orbit::update::{self, Manifest};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use windows::Win32::Foundation::{
    COLORREF, ERROR_ALREADY_EXISTS, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows::Win32::Graphics::Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, EnumDisplayMonitors, FillRect,
    GetMonitorInfoW, HGDIOBJ, HMONITOR, InvalidateRect, MONITOR_DEFAULTTONEAREST, MONITORINFO,
    MonitorFromPoint, MonitorFromWindow, PAINTSTRUCT,
};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{
    CreateMutexW, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, VK_ESCAPE, VK_SHIFT,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MOD_ALT, MOD_CONTROL, RegisterHotKey, UnregisterHotKey, VK_CONTROL, VK_MENU,
};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_ERROR, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
    Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::SetCursorPos;
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CallNextHookEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    DestroyWindow, DispatchMessageW, EnumWindows, FindWindowW, GA_ROOT, GWL_EXSTYLE, GWL_STYLE,
    GetAncestor, GetClassNameW, GetCursorPos, GetForegroundWindow, GetMessageW, GetPropW,
    GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, HC_ACTION, IDC_ARROW,
    IDI_APPLICATION, IDYES, IsHungAppWindow, IsWindow, IsWindowVisible, IsZoomed, KillTimer,
    LoadCursorW, LoadIconW, MB_ICONINFORMATION, MB_OK, MB_YESNO, MF_SEPARATOR, MF_STRING, MSG,
    MSLLHOOKSTRUCT, MessageBoxW, PostMessageW, PostQuitMessage, RegisterClassW, RemovePropW,
    SHOW_WINDOW_CMD, SMTO_ABORTIFHUNG, SMTO_BLOCK, SW_HIDE, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE,
    SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOZORDER, SendMessageTimeoutW, SetForegroundWindow,
    SetLayeredWindowAttributes, SetPropW, SetTimer, SetWindowPlacement, SetWindowPos,
    SetWindowsHookExW, ShowWindow, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu,
    TranslateMessage, UnhookWindowsHookEx, WH_MOUSE_LL, WINDOWPLACEMENT, WM_APP, WM_CLOSE,
    WM_COPYDATA, WM_DESTROY, WM_HOTKEY, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_PAINT, WM_RBUTTONUP, WM_TIMER, WNDCLASSW, WS_CAPTION, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_OVERLAPPED, WS_POPUP, WS_THICKFRAME, WindowFromPoint,
};
use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, LWA_ALPHA, WS_EX_TRANSPARENT};
use windows::core::BOOL;
use windows::core::{PCWSTR, w};

const HOTKEY_ID: i32 = 1;
const SHORTCUT_HOTKEY_BASE: i32 = 100;
const TIMER_ID: usize = 1;
const UPDATE_TIMER_ID: usize = 2;
const TRAY_MESSAGE: u32 = WM_APP + 1;
const OPEN_SETTINGS_MESSAGE: u32 = WM_APP + 2;
const UPDATE_AVAILABLE_MESSAGE: u32 = WM_APP + 3;
const UPDATE_READY_MESSAGE: u32 = WM_APP + 4;
const UPDATE_FAILED_MESSAGE: u32 = WM_APP + 5;
const RELOAD_SETTINGS_MESSAGE: u32 = WM_APP + 6;
const DRAG_EVENT_MESSAGE: u32 = WM_APP + 7;
const WHEEL_EVENT_MESSAGE: u32 = WM_APP + 8;
const IPC_MAGIC: usize = 0x4f52_4254;
const IPC_TIMEOUT_MS: u32 = 5000;
const MAX_IPC_BYTES: usize = 4096;
const RECOVERY_PROPERTY: windows::core::PCWSTR = w!("Orbit.Recovery.6C96F66A");
static HOOK_HOST: AtomicUsize = AtomicUsize::new(0);
#[derive(Clone, Copy)]
struct HookMouseEvent {
    message: u32,
    point: POINT,
}
static HOOK_EVENTS: Mutex<VecDeque<HookMouseEvent>> = Mutex::new(VecDeque::new());

static UPDATE_CHECKING: AtomicBool = AtomicBool::new(false);
static AVAILABLE_UPDATE: Mutex<Option<Manifest>> = Mutex::new(None);
static DOWNLOADED_UPDATE: Mutex<Option<(PathBuf, Manifest)>> = Mutex::new(None);
static UPDATE_ERROR: Mutex<Option<String>> = Mutex::new(None);
static UPDATE_PROMPT_PENDING: AtomicBool = AtomicBool::new(false);
static UPDATE_DOWNLOADING: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
struct Session {
    overlay: Option<HWND>,
    preview: Option<HWND>,
    host: Option<HWND>,
    target: Option<HWND>,
    origin: (i32, i32),
    last_radial_cursor: Option<(i32, i32)>,
    selected: Option<Action>,
    selected_sector: Option<usize>,
    open: bool,
    settings: Settings,
    history: History,
    history_busy: bool,
    shortcuts: Vec<ShortcutCycle>,
    stash: Vec<StashedWindow>,
    hidden: Vec<StashedWindow>,
    trigger_started: Option<Instant>,
    trigger_pending: bool,
    drag: Option<DragState>,
}

#[derive(Clone)]
struct ShortcutCycle {
    ids: Vec<i32>,
    actions: Vec<Action>,
    next: usize,
    last_press: Option<Instant>,
}

#[derive(Clone)]
struct StashedWindow {
    window: HWND,
    placement: windows::Win32::UI::WindowsAndMessaging::WINDOWPLACEMENT,
    process_id: u32,
    token: usize,
    hidden: bool,
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
struct StoredRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
struct StoredPlacement {
    flags: u32,
    show_cmd: u32,
    min_position: (i32, i32),
    max_position: (i32, i32),
    normal_position: StoredRect,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct RecoveryEntry {
    hwnd: isize,
    process_id: u32,
    token: usize,
    placement: StoredPlacement,
    hidden: bool,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct RecoveryJournal {
    windows: Vec<RecoveryEntry>,
}

struct DragState {
    window: HWND,
    start_cursor: POINT,
    last_cursor: POINT,
    start_frame: RECT,
    candidate: Option<Action>,
}

thread_local! { static SESSION: RefCell<Session> = RefCell::new(Session::default()); }

fn with_history<T>(operation: impl FnOnce(&mut History) -> Result<T, String>) -> Result<T, String> {
    let Some(mut history) = SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        if session.history_busy {
            return None;
        }
        session.history_busy = true;
        Some(std::mem::take(&mut session.history))
    }) else {
        return Err("window history is busy".into());
    };
    let result = operation(&mut history);
    SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        session.history = history;
        session.history_busy = false;
    });
    result
}

#[derive(serde::Serialize, serde::Deserialize)]
struct DispatchRequest {
    version: u8,
    target: isize,
    action: Action,
}

/// Dispatch to the resident process when it exists so undo, stash, initial-frame,
/// configured actions, and exclusions all share the same state. Ordinary frame actions
/// still work as one-shot commands when Orbit is not resident.
pub fn dispatch_foreground(action: Action) -> Result<(), String> {
    let target = unsafe { GetForegroundWindow() };
    if target.0.is_null() && !matches!(action, Action::Unstash) {
        return Err("there is no foreground window".into());
    }
    let settings = Settings::load()?;
    if !matches!(action, Action::Unstash) {
        ensure_target(target, &settings)?;
    }
    if let Ok(host) = unsafe { FindWindowW(w!("OrbitWindow"), w!("Orbit")) } {
        let request = DispatchRequest {
            version: 1,
            target: target.0 as isize,
            action,
        };
        let bytes = serde_json::to_vec(&request).map_err(|error| error.to_string())?;
        return send_ipc(host, &bytes);
    }
    if requires_resident(action) {
        return Err("start Orbit to use undo, initial frame, stash, and unstash actions".into());
    }
    execute_action(target, action, &settings)
}

fn send_ipc(host: HWND, bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() || bytes.len() > MAX_IPC_BYTES {
        return Err("Orbit request is too large".into());
    }
    let mut payload = bytes.to_vec();
    let packet = COPYDATASTRUCT {
        dwData: IPC_MAGIC,
        cbData: payload.len() as u32,
        lpData: payload.as_mut_ptr().cast(),
    };
    let mut response = 0usize;
    let sent = unsafe {
        SendMessageTimeoutW(
            host,
            WM_COPYDATA,
            WPARAM(0),
            LPARAM((&packet as *const COPYDATASTRUCT) as isize),
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            IPC_TIMEOUT_MS,
            Some(&mut response),
        )
    };
    if sent.0 == 0 {
        return Err(
            "Orbit did not respond within 5 seconds; the action may not have completed".into(),
        );
    }
    let code = response as isize;
    if code == 1 {
        Ok(())
    } else {
        Err(ipc_error(code))
    }
}

fn ipc_error(code: isize) -> String {
    match code {
        -1 => "the target window is no longer available".into(),
        -2 => "the target application is excluded in Orbit settings".into(),
        -3 => "there is no previous window position to undo".into(),
        -4 => "Orbit has not recorded an initial frame for this window".into(),
        -5 => "there is no stashed window to restore".into(),
        -6 => "the requested action is not supported on Windows".into(),
        -7 => "Orbit could not apply the window action".into(),
        value => format!("Orbit returned an invalid action result ({value})"),
    }
}

pub fn running_status() -> bool {
    unsafe { FindWindowW(w!("OrbitWindow"), w!("Orbit")).is_ok() }
}

pub fn reload_running_settings() -> Result<(), String> {
    let Ok(host) = (unsafe { FindWindowW(w!("OrbitWindow"), w!("Orbit")) }) else {
        return Ok(());
    };
    if SESSION.with(|cell| cell.borrow().host == Some(host)) {
        return reload_settings();
    }
    send_ipc(host, br#"{"version":1,"reload":true}"#)
}

pub fn check_for_updates() -> Result<(), String> {
    let host = unsafe { FindWindowW(w!("OrbitWindow"), w!("Orbit")) }
        .map_err(|_| "start Orbit before checking for updates".to_string())?;
    start_update_check(host, true)
}

#[cfg(test)]
pub fn apply(hwnd: HWND, action: Action, padding: i32) -> Result<(), String> {
    let settings = Settings {
        padding: padding.clamp(0, 100),
        ..Settings::default()
    };
    apply_frame_action(hwnd, action, &settings)
}

fn ensure_target(hwnd: HWND, settings: &Settings) -> Result<(), String> {
    if hwnd.0.is_null()
        || !unsafe { IsWindow(Some(hwnd)).as_bool() && IsWindowVisible(hwnd).as_bool() }
    {
        return Err("target window is no longer available".into());
    }
    ensure_target_identity(hwnd, settings)
}

fn ensure_target_identity(hwnd: HWND, settings: &Settings) -> Result<(), String> {
    if hwnd.0.is_null() || !unsafe { IsWindow(Some(hwnd)).as_bool() } {
        return Err("target window is no longer available".into());
    }
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 };
    if style & WS_EX_TOOLWINDOW.0 != 0 {
        return Err("target is a tool window".into());
    }
    if is_protected_window(hwnd) {
        return Err("system, desktop, and Orbit windows cannot be moved".into());
    }
    if is_excluded(hwnd, settings)? {
        return Err("target application is excluded in Orbit settings".into());
    }
    Ok(())
}

fn is_excluded(hwnd: HWND, settings: &Settings) -> Result<bool, String> {
    if settings.excluded_processes.is_empty() {
        return Ok(false);
    }
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    if pid == 0 {
        return Err("cannot identify target window process".into());
    }
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }
        .map_err(|error| format!("cannot check target against process exclusions: {error}"))?;
    let mut buffer = vec![0u16; 32768];
    let mut length = buffer.len() as u32;
    let result = unsafe {
        QueryFullProcessImageNameW(
            process,
            Default::default(),
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    };
    let _ = unsafe { windows::Win32::Foundation::CloseHandle(process) };
    result.map_err(|error| format!("cannot check target against process exclusions: {error}"))?;
    let image = String::from_utf16_lossy(&buffer[..length as usize]);
    let basename = Path::new(&image)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(&image);
    Ok(settings.excluded_processes.iter().any(|name| {
        let value = name.trim();
        value.eq_ignore_ascii_case(basename)
            || value.eq_ignore_ascii_case(&image)
            || Path::new(value)
                .file_name()
                .and_then(|file| file.to_str())
                .is_some_and(|file| file.eq_ignore_ascii_case(basename))
    }))
}

fn is_protected_window(hwnd: HWND) -> bool {
    let mut class = [0u16; 256];
    let length = unsafe { GetClassNameW(hwnd, &mut class) };
    if length <= 0 {
        return true;
    }
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    pid == unsafe { windows::Win32::System::Threading::GetCurrentProcessId() }
        || matches!(
            String::from_utf16_lossy(&class[..length as usize]).as_str(),
            "Progman" | "WorkerW" | "Shell_TrayWnd" | "OrbitWindow"
        )
}

#[derive(Clone, Copy)]
struct Monitor {
    handle: HMONITOR,
    work: Rect,
    full: Rect,
}

fn monitor_info(handle: HMONITOR) -> Result<Monitor, String> {
    if handle.0.is_null() {
        return Err("cannot find target monitor".into());
    }
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(handle, &mut info).as_bool() } {
        return Err(format!(
            "cannot read monitor work area: {}",
            windows::core::Error::from_thread()
        ));
    }
    Ok(Monitor {
        handle,
        work: Rect {
            left: info.rcWork.left,
            top: info.rcWork.top,
            right: info.rcWork.right,
            bottom: info.rcWork.bottom,
        },
        full: Rect {
            left: info.rcMonitor.left,
            top: info.rcMonitor.top,
            right: info.rcMonitor.right,
            bottom: info.rcMonitor.bottom,
        },
    })
}

unsafe extern "system" fn collect_monitor(
    handle: HMONITOR,
    _dc: windows::Win32::Graphics::Gdi::HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let monitors = unsafe { &mut *(data.0 as *mut Vec<Monitor>) };
    if let Ok(info) = monitor_info(handle) {
        monitors.push(info);
    }
    BOOL(1)
}

fn monitors() -> Vec<Monitor> {
    let mut result = Vec::new();
    let _ = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(collect_monitor),
            LPARAM((&mut result as *mut Vec<Monitor>) as isize),
        )
    };
    result
}

fn monitor_for(hwnd: HWND, settings: &Settings) -> Result<Monitor, String> {
    let handle = if settings.use_screen_with_cursor {
        let mut point = POINT::default();
        if unsafe { GetCursorPos(&mut point) }.is_ok() {
            unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST) }
        } else {
            unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) }
        }
    } else {
        unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) }
    };
    monitor_info(handle)
}

fn target_frame(hwnd: HWND, action: Action, settings: &Settings) -> Result<Rect, String> {
    let current = rect_from_window(hwnd)?;
    let monitor = monitor_for(hwnd, settings)?;
    if action == Action::Fullscreen {
        return Ok(monitor.full);
    }
    if action == Action::MacOSCenter {
        let width = current.width().min(monitor.work.width());
        let height = current.height().min(monitor.work.height());
        let left = monitor.work.left + (monitor.work.width() - width) / 2;
        let top = monitor.work.top + (monitor.work.height() - height) / 4;
        return Ok(Rect {
            left,
            top,
            right: left + width,
            bottom: top + height,
        });
    }
    if action == Action::FillAvailableSpace {
        let mut windows = Vec::new();
        unsafe {
            let _ = EnumWindows(
                Some(collect_window),
                LPARAM((&mut windows as *mut Vec<HWND>) as isize),
            );
        }
        let obstacles: Vec<_> = windows
            .into_iter()
            .filter(|candidate| {
                *candidate != hwnd
                    && unsafe { IsWindowVisible(*candidate).as_bool() }
                    && !is_protected_window(*candidate)
            })
            .filter_map(|candidate| rect_from_window(candidate).ok())
            .collect();
        return Ok(
            orbit::available_space::largest_frame(monitor.work, current, &obstacles)
                .inset(settings.padding),
        );
    }
    if let Action::Custom(index) = action {
        let frame = settings
            .custom_frames
            .get(usize::from(index))
            .ok_or_else(|| format!("custom frame {index} does not exist"))?;
        let x = monitor.work.width();
        let y = monitor.work.height();
        return Ok(Rect {
            left: monitor.work.left + (f64::from(x) * frame.x).round() as i32,
            top: monitor.work.top + (f64::from(y) * frame.y).round() as i32,
            right: monitor.work.left + (f64::from(x) * (frame.x + frame.width)).round() as i32,
            bottom: monitor.work.top + (f64::from(y) * (frame.y + frame.height)).round() as i32,
        }
        .inset(settings.padding));
    }
    Ok(action.frame_with_increment(
        monitor.work,
        current,
        settings.padding,
        settings.size_increment,
    ))
}

fn rect_from_window(hwnd: HWND) -> Result<Rect, String> {
    let mut outer = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut outer) }
        .map_err(|error| format!("cannot read window frame: {error}"))?;
    let mut visible = RECT::default();
    if unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            (&mut visible as *mut RECT).cast(),
            std::mem::size_of::<RECT>() as u32,
        )
    }
    .is_ok()
    {
        Ok(Rect {
            left: visible.left,
            top: visible.top,
            right: visible.right,
            bottom: visible.bottom,
        })
    } else {
        Ok(Rect {
            left: outer.left,
            top: outer.top,
            right: outer.right,
            bottom: outer.bottom,
        })
    }
}

fn set_visible_frame(
    hwnd: HWND,
    target: Rect,
    extra_flags: windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS,
) -> Result<(), String> {
    let mut outer = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut outer) }
        .map_err(|error| format!("cannot read window frame: {error}"))?;
    let mut visible = outer;
    let has_dwm_bounds = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            (&mut visible as *mut RECT).cast(),
            std::mem::size_of::<RECT>() as u32,
        )
    }
    .is_ok();
    let (left_margin, top_margin, right_margin, bottom_margin) = if has_dwm_bounds {
        (
            visible.left - outer.left,
            visible.top - outer.top,
            outer.right - visible.right,
            outer.bottom - visible.bottom,
        )
    } else {
        (0, 0, 0, 0)
    };
    let x = target.left.saturating_sub(left_margin);
    let y = target.top.saturating_sub(top_margin);
    let width = target
        .width()
        .saturating_add(left_margin)
        .saturating_add(right_margin)
        .max(1);
    let height = target
        .height()
        .saturating_add(top_margin)
        .saturating_add(bottom_margin)
        .max(1);
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            x,
            y,
            width,
            height,
            SWP_NOACTIVATE | SWP_NOZORDER | extra_flags,
        )
    }
    .map_err(|error| format!("cannot position window: {error}"))?;
    let actual = rect_from_window(hwnd)?;
    if (actual.left - target.left).abs() > 2
        || (actual.top - target.top).abs() > 2
        || (actual.right - target.right).abs() > 2
        || (actual.bottom - target.bottom).abs() > 2
    {
        return Err("the target window refused the requested frame".into());
    }
    Ok(())
}

fn apply_frame_action(hwnd: HWND, action: Action, settings: &Settings) -> Result<(), String> {
    if !unsafe { IsWindow(Some(hwnd)).as_bool() && IsWindowVisible(hwnd).as_bool() } {
        return Err("target window is no longer available".into());
    }
    if unsafe { IsHungAppWindow(hwnd).as_bool() } {
        return Err("target window is not responding".into());
    }
    if settings.ignore_fullscreen && is_fullscreen(hwnd, monitor_for(hwnd, settings)?) {
        return Ok(());
    }
    match action {
        Action::NoAction => return Ok(()),
        Action::Maximize => {
            unsafe {
                let _ = ShowWindow(hwnd, SW_MAXIMIZE);
            }
            return Ok(());
        }
        Action::Minimize => {
            unsafe {
                let _ = ShowWindow(hwnd, SW_MINIMIZE);
            }
            return Ok(());
        }
        Action::Restore => {
            unsafe {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            return Ok(());
        }
        Action::Hide => return hide_window(hwnd),
        _ => {}
    }
    if unsafe { IsZoomed(hwnd).as_bool() } {
        unsafe {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
    }
    let target = target_frame(hwnd, action, settings)?;
    let old = rect_from_window(hwnd)?;
    set_visible_frame(hwnd, target, Default::default())?;
    if settings.move_cursor_with_window {
        let mut cursor = POINT::default();
        if unsafe { GetCursorPos(&mut cursor) }.is_ok() {
            let width = old.width().max(1);
            let height = old.height().max(1);
            let x = (cursor.x - old.left).clamp(0, width);
            let y = (cursor.y - old.top).clamp(0, height);
            let _ = unsafe {
                SetCursorPos(
                    target.left + x * target.width() / width,
                    target.top + y * target.height() / height,
                )
            };
        }
    }
    if settings.focus_window_on_resize {
        let _ = unsafe { SetForegroundWindow(hwnd) };
    }
    Ok(())
}

fn is_fullscreen(hwnd: HWND, monitor: Monitor) -> bool {
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 };
    if style & (WS_CAPTION.0 | WS_THICKFRAME.0) != 0 {
        return false;
    }
    rect_from_window(hwnd).is_ok_and(|rect| {
        rect.left <= monitor.full.left
            && rect.top <= monitor.full.top
            && rect.right >= monitor.full.right
            && rect.bottom >= monitor.full.bottom
    })
}

fn requires_resident(action: Action) -> bool {
    matches!(
        action,
        Action::Undo
            | Action::InitialFrame
            | Action::Stash
            | Action::StashLeft
            | Action::StashRight
            | Action::StashUp
            | Action::StashDown
            | Action::Unstash
            | Action::Hide
            | Action::Restore
    )
}

fn execute_action(hwnd: HWND, action: Action, settings: &Settings) -> Result<(), String> {
    let hidden_restore = matches!(action, Action::Restore | Action::Undo)
        && SESSION.with(|cell| {
            cell.borrow()
                .hidden
                .iter()
                .any(|entry| entry.window == hwnd)
        });
    if hidden_restore {
        ensure_target_identity(hwnd, settings)?;
    } else if action != Action::Unstash && !(action == Action::Undo && hwnd.0.is_null()) {
        ensure_target(hwnd, settings)?;
    }
    match action {
        Action::Undo => {
            let window = if hwnd.0.is_null() {
                with_history(|history| Ok(history.last_window()))?
            } else {
                Some(hwnd)
            };
            let tracked = window.is_some_and(|window| {
                SESSION.with(|cell| {
                    let session = cell.borrow();
                    session
                        .stash
                        .iter()
                        .chain(&session.hidden)
                        .any(|item| item.window == window)
                })
            });
            with_history(|history| {
                history
                    .undo(if hwnd.0.is_null() { None } else { Some(hwnd) })
                    .map_err(|error| error.to_string())
            })?;
            if tracked && let Some(window) = window {
                let hidden = SESSION.with(|cell| {
                    cell.borrow()
                        .hidden
                        .iter()
                        .any(|item| item.window == window)
                });
                clear_recovery(window, hidden)?;
            }
            return Ok(());
        }
        Action::InitialFrame => {
            return with_history(|history| {
                if !history.has_initial_frame(hwnd) {
                    history.capture_initial(hwnd)?;
                }
                history.initial_frame(hwnd)
            });
        }
        Action::Unstash => return unstash(settings),
        Action::Stash
        | Action::StashLeft
        | Action::StashRight
        | Action::StashUp
        | Action::StashDown => return stash(hwnd, action, settings),
        Action::Hide => return hide_window(hwnd),
        Action::Restore if hidden_restore => return restore_hidden_window(hwnd),
        Action::MinimizeOthers => {
            let mut windows = Vec::new();
            unsafe {
                let _ = EnumWindows(
                    Some(collect_window),
                    LPARAM((&mut windows as *mut Vec<HWND>) as isize),
                );
            }
            for window in windows {
                if window != hwnd
                    && unsafe { IsWindowVisible(window).as_bool() }
                    && !is_protected_window(window)
                    && !is_excluded(window, settings).unwrap_or(true)
                {
                    unsafe {
                        let _ = ShowWindow(window, SW_MINIMIZE);
                    }
                }
            }
            return Ok(());
        }
        Action::NextMonitor
        | Action::PreviousMonitor
        | Action::MoveToMonitorLeft
        | Action::MoveToMonitorRight
        | Action::MoveToMonitorUp
        | Action::MoveToMonitorDown => {
            return move_to_monitor(hwnd, action);
        }
        Action::FocusLeft
        | Action::FocusRight
        | Action::FocusUp
        | Action::FocusDown
        | Action::FocusNextInStack => {
            return focus_window(hwnd, action, settings);
        }
        _ => {}
    }
    if action == Action::NoAction {
        return Ok(());
    }
    let before = history::placement(hwnd)?;
    with_history(|history| history.record(hwnd, before))?;
    apply_frame_action(hwnd, action, settings)?;
    Ok(())
}

fn stash(hwnd: HWND, action: Action, settings: &Settings) -> Result<(), String> {
    let before = history::placement(hwnd)?;
    let current = rect_from_window(hwnd)?;
    let monitor = monitor_info(unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) })?;
    let padding = settings
        .stash_visible_padding
        .clamp(0, current.width().min(current.height()).max(1) - 1);
    let (x, y) = match action {
        Action::StashRight => (monitor.full.right - padding, current.top),
        Action::StashUp => (current.left, monitor.full.top - current.height() + padding),
        Action::StashDown => (current.left, monitor.full.bottom - padding),
        _ => (monitor.full.left - current.width() + padding, current.top),
    };
    let already_stashed =
        SESSION.with(|cell| cell.borrow().stash.iter().any(|item| item.window == hwnd));
    if already_stashed {
        return Err("this window is already stashed".into());
    }
    with_history(|history| history.record(hwnd, before))?;
    let _tracked = track_recoverable_window(hwnd, before, false)?;
    set_visible_frame(
        hwnd,
        Rect {
            left: x,
            top: y,
            right: x.saturating_add(current.width()),
            bottom: y.saturating_add(current.height()),
        },
        Default::default(),
    )
    .map_err(|error| {
        let _ = clear_recovery(hwnd, false);
        format!("cannot stash window: {error}")
    })
}

fn unstash(settings: &Settings) -> Result<(), String> {
    let entry = SESSION
        .with(|cell| cell.borrow().stash.last().cloned())
        .ok_or("there is no stashed window to restore")?;
    if !unsafe { IsWindow(Some(entry.window)).as_bool() } {
        return Err("stashed window is no longer available".into());
    }
    if !recovery_entry_is_current(&entry) {
        return Err("stashed window identity changed and cannot be safely restored".into());
    }
    ensure_target_identity(entry.window, settings)?;
    restore_placement(entry.window, &entry.placement)
        .map_err(|error| format!("cannot restore stashed window: {error}"))?;
    clear_recovery(entry.window, false)?;
    if !unsafe { SetForegroundWindow(entry.window).as_bool() } {
        return Err("window was restored, but Windows did not allow it to take focus".into());
    }
    Ok(())
}

fn hide_window(hwnd: HWND) -> Result<(), String> {
    let before = history::placement(hwnd)?;
    if SESSION.with(|cell| cell.borrow().hidden.iter().any(|item| item.window == hwnd)) {
        return Err("this window is already hidden by Orbit".into());
    }
    with_history(|history| history.record(hwnd, before))?;
    let _tracked = track_recoverable_window(hwnd, before, true)?;
    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
    Ok(())
}

fn restore_hidden_window(hwnd: HWND) -> Result<(), String> {
    let entry = SESSION
        .with(|cell| {
            cell.borrow()
                .hidden
                .iter()
                .rev()
                .find(|entry| entry.window == hwnd)
                .cloned()
        })
        .ok_or("window is not hidden by Orbit")?;
    if !unsafe { IsWindow(Some(hwnd)).as_bool() } {
        return Err("hidden window is no longer available".into());
    }
    if !recovery_entry_is_current(&entry) {
        return Err("hidden window identity changed and cannot be safely restored".into());
    }
    let settings = SESSION.with(|cell| cell.borrow().settings.clone());
    ensure_target_identity(hwnd, &settings)?;
    restore_placement(hwnd, &entry.placement)
        .map_err(|error| format!("cannot restore hidden window: {error}"))?;
    clear_recovery(hwnd, true)
}

fn restore_placement(hwnd: HWND, placement: &WINDOWPLACEMENT) -> Result<(), String> {
    if placement.showCmd > 11 {
        return Err("stored window show state is invalid".into());
    }
    unsafe { SetWindowPlacement(hwnd, placement) }.map_err(|error| error.to_string())?;
    unsafe {
        let _ = ShowWindow(hwnd, SHOW_WINDOW_CMD(placement.showCmd as i32));
    }
    if placement.showCmd != 0 && !unsafe { IsWindowVisible(hwnd).as_bool() } {
        return Err("Windows kept the restored window invisible".into());
    }
    Ok(())
}

fn clear_recovery(hwnd: HWND, hidden: bool) -> Result<(), String> {
    let token = SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        let entries = if hidden {
            &mut session.hidden
        } else {
            &mut session.stash
        };
        let index = entries
            .iter()
            .rposition(|entry| entry.window == hwnd)
            .ok_or("window is not tracked by Orbit")?;
        Ok::<_, String>(entries.remove(index).token)
    })?;
    if unsafe { GetPropW(hwnd, RECOVERY_PROPERTY).0 as usize } == token {
        let _ = unsafe { RemovePropW(hwnd, RECOVERY_PROPERTY) };
    }
    persist_recovery()
}

unsafe extern "system" fn collect_window(hwnd: HWND, data: LPARAM) -> BOOL {
    unsafe {
        (&mut *(data.0 as *mut Vec<HWND>)).push(hwnd);
    }
    BOOL(1)
}

fn move_to_monitor(hwnd: HWND, action: Action) -> Result<(), String> {
    let all = monitors();
    if all.is_empty() {
        return Err("no monitors are available".into());
    }
    let current_handle = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    let index = all
        .iter()
        .position(|monitor| monitor.handle == current_handle)
        .unwrap_or(0);
    let current = all[index];
    let destination = match action {
        Action::NextMonitor => all[(index + 1) % all.len()],
        Action::PreviousMonitor => all[(index + all.len() - 1) % all.len()],
        _ => {
            let center = current.work.center();

            all.iter()
                .copied()
                .filter_map(|monitor| {
                    if monitor.handle == current.handle {
                        return None;
                    }
                    let other = monitor.work.center();
                    let dx = other.0 - center.0;
                    let dy = other.1 - center.1;
                    let (primary, secondary) = match action {
                        Action::MoveToMonitorLeft if dx < 0 => (-dx, dy.abs()),
                        Action::MoveToMonitorRight if dx > 0 => (dx, dy.abs()),
                        Action::MoveToMonitorUp if dy < 0 => (-dy, dx.abs()),
                        Action::MoveToMonitorDown if dy > 0 => (dy, dx.abs()),
                        _ => return None,
                    };
                    Some((primary + secondary * 2, monitor))
                })
                .min_by_key(|(score, _)| *score)
                .map(|(_, monitor)| monitor)
                .ok_or("there is no monitor in that direction")?
        }
    };
    let old = rect_from_window(hwnd)?;
    let width = old.width().min(destination.work.width()).max(1);
    let height = old.height().min(destination.work.height()).max(1);
    let center = old.center();
    let x = (center.0 - width / 2).clamp(destination.work.left, destination.work.right - width);
    let y = (center.1 - height / 2).clamp(destination.work.top, destination.work.bottom - height);
    let before = history::placement(hwnd)?;
    let target = Rect {
        left: x,
        top: y,
        right: x.saturating_add(width),
        bottom: y.saturating_add(height),
    };
    with_history(|history| history.record(hwnd, before))?;
    set_visible_frame(hwnd, target, Default::default())
}

fn focus_window(hwnd: HWND, action: Action, settings: &Settings) -> Result<(), String> {
    let mut windows = Vec::new();
    unsafe {
        EnumWindows(
            Some(collect_window),
            LPARAM((&mut windows as *mut Vec<HWND>) as isize),
        )
    }
    .map_err(|error| format!("cannot enumerate windows: {error}"))?;
    let target_rect = rect_from_window(hwnd)?;
    let origin = target_rect.center();
    let mut candidates = Vec::new();
    for (z_order, candidate) in windows.into_iter().enumerate() {
        if candidate == hwnd
            || !unsafe { IsWindowVisible(candidate).as_bool() }
            || is_protected_window(candidate)
            || is_excluded(candidate, settings).unwrap_or(true)
        {
            continue;
        }
        let style = unsafe { GetWindowLongPtrW(candidate, GWL_EXSTYLE) as u32 };
        if style & WS_EX_TOOLWINDOW.0 != 0 {
            continue;
        }
        let Ok(rect) = rect_from_window(candidate) else {
            continue;
        };
        let point = rect.center();
        let dx = point.0 - origin.0;
        let dy = point.1 - origin.1;
        let (primary, secondary) = match action {
            Action::FocusLeft if dx < 0 => (-dx, dy.abs()),
            Action::FocusRight if dx > 0 => (dx, dy.abs()),
            Action::FocusUp if dy < 0 => (-dy, dx.abs()),
            Action::FocusDown if dy > 0 => (dy, dx.abs()),
            Action::FocusNextInStack => (z_order as i32, 0),
            _ => continue,
        };
        candidates.push((primary + secondary * 2, z_order, candidate));
        if action == Action::FocusNextInStack {
            break;
        }
    }
    let target = candidates
        .into_iter()
        .min_by_key(|(score, order, _)| (*score, *order))
        .map(|(_, _, candidate)| candidate)
        .ok_or("no eligible window is available in that direction")?;
    if !unsafe { SetForegroundWindow(target).as_bool() } {
        return Err("Windows did not allow the target window to take focus".into());
    }
    Ok(())
}

fn hotkey_modifiers(
    hotkey: Hotkey,
) -> windows::Win32::UI::Input::KeyboardAndMouse::HOT_KEY_MODIFIERS {
    let mut modifiers = MOD_NOREPEAT;
    if hotkey.control {
        modifiers |= MOD_CONTROL;
    }
    if hotkey.alt {
        modifiers |= MOD_ALT;
    }
    if hotkey.shift {
        modifiers |= MOD_SHIFT;
    }
    if hotkey.win {
        modifiers |= MOD_WIN;
    }
    modifiers
}

fn register_hotkeys(host: HWND, settings: &Settings) -> Result<Vec<ShortcutCycle>, String> {
    let mut registered = Vec::new();
    let trigger = unsafe {
        RegisterHotKey(
            Some(host),
            HOTKEY_ID,
            hotkey_modifiers(settings.trigger),
            u32::from(settings.trigger.key),
        )
    };
    if let Err(error) = trigger {
        return Err(format!("cannot register the Orbit trigger hotkey: {error}"));
    }
    registered.push(HOTKEY_ID);
    let mut cycles = Vec::new();
    for (index, shortcut) in settings.shortcuts.iter().enumerate() {
        let mut ids = Vec::new();
        let id = SHORTCUT_HOTKEY_BASE + index as i32;
        if let Err(error) = unsafe {
            RegisterHotKey(
                Some(host),
                id,
                hotkey_modifiers(shortcut.hotkey),
                u32::from(shortcut.hotkey.key),
            )
        } {
            for old in registered {
                let _ = unsafe { UnregisterHotKey(Some(host), old) };
            }
            return Err(format!(
                "cannot register shortcut {}: {error}",
                shortcut.hotkey.key
            ));
        }
        ids.push(id);
        registered.push(id);
        if settings.cycle_backwards_on_shift && shortcut.actions.len() > 1 {
            let mut alternate = shortcut.hotkey;
            alternate.shift = !alternate.shift;
            let reverse_id = SHORTCUT_HOTKEY_BASE + 2048 + index as i32;
            if let Err(error) = unsafe {
                RegisterHotKey(
                    Some(host),
                    reverse_id,
                    hotkey_modifiers(alternate),
                    u32::from(alternate.key),
                )
            } {
                for old in registered {
                    let _ = unsafe { UnregisterHotKey(Some(host), old) };
                }
                return Err(format!(
                    "cannot register shifted shortcut {}: {error}",
                    shortcut.hotkey.key
                ));
            }
            ids.push(reverse_id);
            registered.push(reverse_id);
        }
        cycles.push(ShortcutCycle {
            ids,
            actions: shortcut.actions.clone(),
            next: 0,
            last_press: None,
        });
    }
    Ok(cycles)
}

fn unregister_hotkeys(host: HWND) {
    let ids = SESSION.with(|cell| {
        cell.borrow()
            .shortcuts
            .iter()
            .flat_map(|shortcut| shortcut.ids.iter().copied())
            .collect::<Vec<_>>()
    });
    for id in ids {
        let _ = unsafe { UnregisterHotKey(Some(host), id) };
    }
    let _ = unsafe { UnregisterHotKey(Some(host), HOTKEY_ID) };
}

fn register_shortcut_action(hwnd: HWND, id: i32) {
    let (target, action, settings) = SESSION
        .with(|cell| {
            let mut session = cell.borrow_mut();
            let timeout_ms = session.settings.cycle_timeout_ms;
            let cycle_backwards = session.settings.cycle_backwards_on_shift;
            let settings = session.settings.clone();
            let index = session
                .shortcuts
                .iter()
                .position(|cycle| cycle.ids.contains(&id))?;
            let now = Instant::now();
            let cycle = &mut session.shortcuts[index];
            if cycle
                .last_press
                .is_some_and(|last| now.duration_since(last).as_millis() > u128::from(timeout_ms))
            {
                cycle.next = 0;
            }
            let reverse = cycle_backwards && cycle.ids.get(1) == Some(&id);
            let index = if reverse {
                (cycle.next + cycle.actions.len() - 1) % cycle.actions.len()
            } else {
                cycle.next % cycle.actions.len()
            };
            let action = cycle.actions[index];
            cycle.next = if reverse {
                index
            } else {
                (index + 1) % cycle.actions.len()
            };
            cycle.last_press = Some(now);
            Some((unsafe { GetForegroundWindow() }, action, settings))
        })
        .unwrap_or_else(|| {
            (
                HWND(std::ptr::null_mut()),
                Action::NoAction,
                Settings::default(),
            )
        });
    if target.0.is_null() {
        return;
    }
    if let Err(error) = execute_action(target, action, &settings) {
        notify_error(hwnd, &error);
    }
    let _ = hwnd;
}

fn input_key_down(key: u16) -> bool {
    unsafe { GetAsyncKeyState(i32::from(key)) as u16 & 0x8000 != 0 }
}

fn trigger_held(hotkey: Hotkey) -> bool {
    input_key_down(hotkey.key)
        && (!hotkey.control
            || unsafe { GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000 != 0 })
        && (!hotkey.alt || unsafe { GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000 != 0 })
        && (!hotkey.shift || unsafe { GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000 != 0 })
        && (!hotkey.win || input_key_down(0x5B) || input_key_down(0x5C))
}

fn begin_radial(host: HWND) {
    if SESSION.with(|cell| cell.borrow().open) {
        return;
    }
    let settings = SESSION.with(|cell| cell.borrow().settings.clone());
    let mut target = unsafe { GetForegroundWindow() };
    let mut cursor = POINT::default();
    if unsafe { GetCursorPos(&mut cursor) }.is_err() {
        return;
    }
    if settings.resize_window_under_cursor {
        let under = unsafe { WindowFromPoint(cursor) };
        if !under.0.is_null() {
            let root = unsafe { GetAncestor(under, GA_ROOT) };
            if !root.0.is_null() {
                target = root;
            } else {
                target = under;
            }
        }
    }
    if let Err(error) = ensure_target(target, &settings) {
        notify_error(host, &error);
        return;
    }
    let monitor = match monitor_for(target, &settings) {
        Ok(monitor) => monitor,
        Err(error) => {
            notify_error(host, &error);
            return;
        }
    };
    let origin = if settings.lock_radial_menu_to_center {
        monitor.work.center()
    } else {
        (cursor.x, cursor.y)
    };
    if let Err(error) = with_history(|history| history.capture_initial(target)) {
        notify_error(host, &error);
        return;
    }
    let overlay = SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        session.target = Some(target);
        session.origin = origin;
        session.last_radial_cursor = None;
        session.selected = None;
        session.selected_sector = None;
        session.open = true;
        session.trigger_pending = false;
        session.trigger_started = None;
        session.overlay
    });
    if let Some(overlay) = overlay {
        let dpi = unsafe { windows::Win32::UI::HiDpi::GetDpiForWindow(target) }.max(96);
        let size = crate::radial::window_size_px(&settings, dpi);
        let _ = unsafe {
            SetWindowPos(
                overlay,
                None,
                origin.0 - size / 2,
                origin.1 - size / 2,
                size,
                size,
                SWP_NOACTIVATE | SWP_NOZORDER,
            )
        };
        if settings.radial_menu_visible {
            if let Err(error) =
                crate::radial::draw_with_settings_and_sector(overlay, None, None, &settings, dpi)
            {
                notify_error(host, &error);
            }
            unsafe {
                let _ = ShowWindow(overlay, SW_SHOWNOACTIVATE);
            }
        }
        unsafe {
            let _ = SetTimer(Some(host), TIMER_ID, 16, None);
        }
    }
}

pub fn run() -> Result<(), String> {
    let settings = Settings::load()?;
    let instance_mutex =
        unsafe { CreateMutexW(None, false, w!("Local\\Orbit.WindowManager.6C96F66A")) }
            .map_err(|error| format!("cannot create Orbit instance mutex: {error}"))?;
    let _instance_guard = InstanceMutex(instance_mutex);
    if unsafe { windows::Win32::Foundation::GetLastError() } == ERROR_ALREADY_EXISTS {
        return Err("Orbit is already running".into());
    }
    if unsafe { FindWindowW(w!("OrbitWindow"), w!("Orbit")) }.is_ok() {
        return Err("Orbit is already running".into());
    }
    let recovery_error = recover_after_crash().err();
    let module = unsafe { GetModuleHandleW(None) }.map_err(|e| e.to_string())?;
    let instance = HINSTANCE(module.0);
    let class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: instance,
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.map_err(|e| e.to_string())?,
        hIcon: unsafe { LoadIconW(Some(instance), PCWSTR(std::ptr::without_provenance(1))) }
            .unwrap_or(unsafe { LoadIconW(None, IDI_APPLICATION) }.map_err(|e| e.to_string())?),
        lpszClassName: w!("OrbitWindow"),
        ..Default::default()
    };
    if unsafe { RegisterClassW(&class) } == 0 {
        return Err(format!(
            "cannot register window class: {}",
            windows::core::Error::from_thread()
        ));
    }
    let host = unsafe {
        CreateWindowExW(
            Default::default(),
            w!("OrbitWindow"),
            w!("Orbit"),
            WS_OVERLAPPED,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(instance),
            None,
        )
    }
    .map_err(|e| e.to_string())?;
    let preview = unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT,
            w!("OrbitWindow"),
            w!("Orbit preview"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(instance),
            None,
        )
    }
    .map_err(|e| e.to_string())?;
    unsafe {
        SetLayeredWindowAttributes(preview, COLORREF(0), settings.preview_opacity, LWA_ALPHA)
    }
    .map_err(|e| e.to_string())?;
    let overlay = unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w!("OrbitWindow"),
            w!("Orbit radial menu"),
            WS_POPUP,
            0,
            0,
            crate::radial::window_size_px(&settings, 96),
            crate::radial::window_size_px(&settings, 96),
            None,
            None,
            Some(instance),
            None,
        )
    }
    .map_err(|e| e.to_string())?;
    crate::radial::draw_with_settings(overlay, None, &settings, 96)?;
    SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        session.overlay = Some(overlay);
        session.preview = Some(preview);
        session.host = Some(host);
        session.settings = settings.clone();
    });
    let icon = class.hIcon;
    let mut tray = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: host,
        uID: 1,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: TRAY_MESSAGE,
        hIcon: icon,
        ..Default::default()
    };
    let tip: Vec<u16> = "Orbit - Ctrl+Alt+Space"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    tray.szTip[..tip.len()].copy_from_slice(&tip);
    if !unsafe { Shell_NotifyIconW(NIM_ADD, &tray).as_bool() } {
        return Err("cannot create notification icon".into());
    }
    if let Some(error) = recovery_error.as_deref() {
        notify_error(host, error);
    }
    let shortcuts = match register_hotkeys(host, &settings) {
        Ok(shortcuts) => shortcuts,
        Err(error) => {
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &tray);
                let _ = DestroyWindow(overlay);
                let _ = DestroyWindow(preview);
                let _ = DestroyWindow(host);
            }
            return Err(error);
        }
    };
    SESSION.with(|cell| cell.borrow_mut().shortcuts = shortcuts);
    HOOK_HOST.store(host.0 as usize, Ordering::Relaxed);
    let mouse_hook =
        match unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), Some(instance), 0) } {
            Ok(hook) => hook,
            Err(error) => {
                HOOK_HOST.store(0, Ordering::Relaxed);
                unregister_hotkeys(host);
                unsafe {
                    let _ = Shell_NotifyIconW(NIM_DELETE, &tray);
                    let _ = DestroyWindow(overlay);
                    let _ = DestroyWindow(preview);
                    let _ = DestroyWindow(host);
                }
                return Err(format!("cannot install Orbit mouse hook: {error}"));
            }
        };
    if settings.updates_enabled && update::configured().is_some() {
        let _ = start_update_check(host, false);
        unsafe {
            SetTimer(Some(host), UPDATE_TIMER_ID, 6 * 60 * 60 * 1000, None);
        }
    }
    let mut msg = MSG::default();
    loop {
        let result = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if result.0 == 0 {
            break;
        }
        if result.0 == -1 {
            return Err(format!(
                "message loop failed: {}",
                windows::core::Error::from_thread()
            ));
        }
        if settings_window::handle_dialog_message(&msg) {
            continue;
        }
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    unregister_hotkeys(host);
    let _ = unsafe { KillTimer(Some(host), UPDATE_TIMER_ID) };
    HOOK_HOST.store(0, Ordering::Relaxed);
    let _ = unsafe { UnhookWindowsHookEx(mouse_hook) };
    if let Err(error) = restore_recoverable_on_exit() {
        notify_error(host, &error);
    }
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &tray);
    }
    let _ = unsafe { DestroyWindow(overlay) };
    let _ = unsafe { DestroyWindow(preview) };
    let history = SESSION.with(|cell| std::mem::take(&mut cell.borrow_mut().history));
    drop(history);
    let _ = unsafe { DestroyWindow(host) };
    Ok(())
}

struct InstanceMutex(HANDLE);
impl Drop for InstanceMutex {
    fn drop(&mut self) {
        let _ = unsafe { windows::Win32::Foundation::CloseHandle(self.0) };
    }
}

fn store_placement(value: WINDOWPLACEMENT) -> StoredPlacement {
    StoredPlacement {
        flags: value.flags.0,
        show_cmd: value.showCmd,
        min_position: (value.ptMinPosition.x, value.ptMinPosition.y),
        max_position: (value.ptMaxPosition.x, value.ptMaxPosition.y),
        normal_position: StoredRect {
            left: value.rcNormalPosition.left,
            top: value.rcNormalPosition.top,
            right: value.rcNormalPosition.right,
            bottom: value.rcNormalPosition.bottom,
        },
    }
}

fn load_placement(value: StoredPlacement) -> WINDOWPLACEMENT {
    WINDOWPLACEMENT {
        length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
        flags: windows::Win32::UI::WindowsAndMessaging::WINDOWPLACEMENT_FLAGS(value.flags),
        showCmd: value.show_cmd,
        ptMinPosition: POINT {
            x: value.min_position.0,
            y: value.min_position.1,
        },
        ptMaxPosition: POINT {
            x: value.max_position.0,
            y: value.max_position.1,
        },
        rcNormalPosition: RECT {
            left: value.normal_position.left,
            top: value.normal_position.top,
            right: value.normal_position.right,
            bottom: value.normal_position.bottom,
        },
    }
}

fn recovery_file() -> Result<PathBuf, String> {
    Settings::path()?
        .parent()
        .map(|parent| parent.join("recovery.json"))
        .ok_or_else(|| "settings path has no parent".into())
}

fn persist_recovery() -> Result<(), String> {
    let entries = SESSION.with(|cell| {
        let session = cell.borrow();
        session
            .stash
            .iter()
            .map(|item| RecoveryEntry {
                hwnd: item.window.0 as isize,
                process_id: item.process_id,
                token: item.token,
                placement: store_placement(item.placement),
                hidden: false,
            })
            .chain(session.hidden.iter().map(|item| RecoveryEntry {
                hwnd: item.window.0 as isize,
                process_id: item.process_id,
                token: item.token,
                placement: store_placement(item.placement),
                hidden: true,
            }))
            .collect::<Vec<_>>()
    });
    let path = recovery_file()?;
    let parent = path.parent().ok_or("recovery path has no parent")?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create recovery directory: {error}"))?;
    if entries.is_empty() {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("cannot clear recovery journal: {error}")),
        }
        return Ok(());
    }
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("cannot create recovery journal: {error}"))?;
    serde_json::to_writer(&mut temp, &RecoveryJournal { windows: entries })
        .map_err(|error| error.to_string())?;
    temp.write_all(b"\n").map_err(|error| error.to_string())?;
    temp.as_file()
        .sync_all()
        .map_err(|error| error.to_string())?;
    temp.persist(&path)
        .map_err(|error| format!("cannot replace recovery journal: {error}"))?;
    Ok(())
}

fn track_recoverable_window(
    hwnd: HWND,
    placement: WINDOWPLACEMENT,
    hidden: bool,
) -> Result<StashedWindow, String> {
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    if pid == 0 {
        return Err("cannot identify window process for recovery".into());
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as usize;
    let token = nonce.wrapping_add(hwnd.0 as usize).max(1);
    unsafe { SetPropW(hwnd, RECOVERY_PROPERTY, Some(HANDLE(token as *mut _))) }
        .map_err(|error| format!("cannot track window recovery: {error}"))?;
    let entry = StashedWindow {
        window: hwnd,
        placement,
        process_id: pid,
        token,
        hidden,
    };
    SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        if hidden {
            session.hidden.push(StashedWindow { ..entry })
        } else {
            session.stash.push(StashedWindow { ..entry })
        }
    });
    if let Err(error) = persist_recovery() {
        SESSION.with(|cell| {
            let mut session = cell.borrow_mut();
            session.stash.retain(|item| item.token != token);
            session.hidden.retain(|item| item.token != token);
        });
        let _ = unsafe { RemovePropW(hwnd, RECOVERY_PROPERTY) };
        return Err(error);
    }
    Ok(entry)
}

fn recovery_entry_is_current(entry: &StashedWindow) -> bool {
    if !unsafe { IsWindow(Some(entry.window)).as_bool() } {
        return false;
    }
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(entry.window, Some(&mut pid));
    }
    pid == entry.process_id
        && unsafe { GetPropW(entry.window, RECOVERY_PROPERTY).0 as usize == entry.token }
}

fn recover_after_crash() -> Result<(), String> {
    let path = recovery_file()?;
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("cannot read recovery journal: {error}")),
    };
    let journal = match serde_json::from_slice::<RecoveryJournal>(&bytes) {
        Ok(journal) => journal,
        Err(error) => {
            let quarantine = path.with_extension(format!(
                "corrupt-{}.json",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            ));
            fs::rename(&path, &quarantine).map_err(|rename_error| {
                format!("invalid recovery journal ({error}); cannot quarantine it: {rename_error}")
            })?;
            return Err(format!(
                "invalid recovery journal was preserved at {}: {error}",
                quarantine.display()
            ));
        }
    };
    let mut pending = Vec::new();
    for entry in journal.windows {
        let hwnd = HWND(entry.hwnd as *mut _);
        if !unsafe { IsWindow(Some(hwnd)).as_bool() } {
            continue;
        }
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
        }
        let marker = unsafe { GetPropW(hwnd, RECOVERY_PROPERTY) }.0 as usize;
        if pid == entry.process_id && marker == entry.token {
            let placement = load_placement(entry.placement);
            if unsafe { SetWindowPlacement(hwnd, &placement) }.is_ok() {
                let _ = unsafe { RemovePropW(hwnd, RECOVERY_PROPERTY) };
            } else {
                pending.push(entry);
            }
        }
    }
    SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        for entry in &pending {
            let window = StashedWindow {
                window: HWND(entry.hwnd as *mut _),
                placement: load_placement(entry.placement),
                process_id: entry.process_id,
                token: entry.token,
                hidden: entry.hidden,
            };
            if entry.hidden {
                session.hidden.push(window);
            } else {
                session.stash.push(window);
            }
        }
    });
    persist_recovery_journal(&path, pending)
}

fn restore_recoverable_on_exit() -> Result<(), String> {
    let entries = SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        std::mem::take(&mut session.hidden)
            .into_iter()
            .chain(std::mem::take(&mut session.stash))
            .collect::<Vec<_>>()
    });
    let mut pending = Vec::new();
    for entry in entries {
        if recovery_entry_is_current(&entry) {
            if unsafe { SetWindowPlacement(entry.window, &entry.placement) }.is_ok() {
                let _ = unsafe { RemovePropW(entry.window, RECOVERY_PROPERTY) };
            } else {
                pending.push(entry);
            }
        }
    }
    let failed_count = pending.len();
    SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        for entry in pending {
            if entry.hidden {
                session.hidden.push(entry);
            } else {
                session.stash.push(entry);
            }
        }
    });
    persist_recovery()?;
    if failed_count > 0 {
        return Err(format!(
            "Cannot close Orbit: {failed_count} windows still need recovery. Close those applications or retry."
        ));
    }
    Ok(())
}

fn persist_recovery_journal(path: &Path, entries: Vec<RecoveryEntry>) -> Result<(), String> {
    let parent = path.parent().ok_or("recovery path has no parent")?;
    if entries.is_empty() {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("cannot clear recovery journal: {error}")),
        }
        return Ok(());
    }
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("cannot create recovery journal: {error}"))?;
    serde_json::to_writer(&mut temp, &RecoveryJournal { windows: entries })
        .map_err(|error| error.to_string())?;
    temp.write_all(b"\n").map_err(|error| error.to_string())?;
    temp.as_file()
        .sync_all()
        .map_err(|error| error.to_string())?;
    temp.persist(path)
        .map_err(|error| format!("cannot replace recovery journal: {error}"))?;
    Ok(())
}

pub fn quit_running() -> Result<(), String> {
    let Ok(hwnd) = (unsafe { FindWindowW(w!("OrbitWindow"), w!("Orbit")) }) else {
        recover_after_crash()?;
        return restore_recoverable_on_exit();
    };
    let mut process_id = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }
    let process = unsafe {
        OpenProcess(
            windows::Win32::System::Threading::PROCESS_SYNCHRONIZE,
            false,
            process_id,
        )
    }
    .map_err(|e| format!("cannot wait for Orbit to close: {e}"))?;
    let result = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) }
        .map_err(|e| format!("cannot close Orbit: {e}"));
    let result = result.and_then(|()| {
        if process_id == unsafe { windows::Win32::System::Threading::GetCurrentProcessId() }
            || unsafe { windows::Win32::System::Threading::WaitForSingleObject(process, 10000) }
                == windows::Win32::Foundation::WAIT_OBJECT_0
        {
            Ok(())
        } else {
            Err("Orbit did not finish closing within 10 seconds".into())
        }
    });
    unsafe {
        let _ = windows::Win32::Foundation::CloseHandle(process);
    }
    result
}

pub fn open_running_settings() -> Result<(), String> {
    let hwnd = unsafe { FindWindowW(w!("OrbitWindow"), w!("Orbit")) }
        .map_err(|_| "Orbit is not running".to_string())?;
    unsafe { PostMessageW(Some(hwnd), OPEN_SETTINGS_MESSAGE, WPARAM(0), LPARAM(0)) }
        .map_err(|e| format!("cannot open settings: {e}"))
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CLOSE => {
            if SESSION.with(|cell| cell.borrow().host == Some(hwnd)) {
                finish_session(hwnd, false);
                if let Err(error) = restore_recoverable_on_exit() {
                    notify_error(hwnd, &error);
                } else {
                    unsafe {
                        PostQuitMessage(0);
                    }
                }
            } else {
                unsafe {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                }
            }
            LRESULT(0)
        }
        WM_COPYDATA => LRESULT(handle_copydata(lparam)),
        UPDATE_AVAILABLE_MESSAGE => {
            if let Some(manifest) = AVAILABLE_UPDATE
                .lock()
                .ok()
                .and_then(|mut slot| slot.take())
            {
                let notes: String = manifest.notes.chars().take(500).collect();
                let text = format!(
                    "Orbit {} is available.\n\n{}\n\nDownload and install now?",
                    manifest.version, notes
                );
                let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
                if unsafe {
                    MessageBoxW(
                        Some(hwnd),
                        PCWSTR(wide.as_ptr()),
                        w!("Orbit update"),
                        MB_YESNO | MB_ICONINFORMATION,
                    )
                } == IDYES
                {
                    start_update_download(hwnd, manifest);
                }
            }
            UPDATE_PROMPT_PENDING.store(false, Ordering::Release);
            LRESULT(0)
        }
        UPDATE_READY_MESSAGE => {
            if let Some((path, manifest)) = DOWNLOADED_UPDATE
                .lock()
                .ok()
                .and_then(|mut slot| slot.take())
            {
                let result = update::verify_download(&path, &manifest);
                if let Err(error) = result {
                    notify_error(hwnd, &error);
                    UPDATE_DOWNLOADING.store(false, Ordering::Release);
                    return LRESULT(0);
                }
                use std::os::windows::process::CommandExt;
                match std::process::Command::new(&path)
                    .arg("/UPDATE=1")
                    .creation_flags(0x08000000)
                    .spawn()
                {
                    Ok(mut child) => {
                        // The installer closes Orbit only after the user proceeds.
                        // Canceling its welcome page leaves window management running.
                        std::thread::spawn(move || {
                            let _ = child.wait();
                            UPDATE_DOWNLOADING.store(false, Ordering::Release);
                        });
                    }
                    Err(error) => {
                        notify_error(
                            hwnd,
                            &format!("Cannot launch the update installer: {error}"),
                        );
                        UPDATE_DOWNLOADING.store(false, Ordering::Release);
                    }
                }
            }
            LRESULT(0)
        }
        UPDATE_FAILED_MESSAGE => {
            if let Some(error) = UPDATE_ERROR.lock().ok().and_then(|mut slot| slot.take()) {
                if wparam.0 == 1 {
                    let wide: Vec<u16> = error.encode_utf16().chain(Some(0)).collect();
                    unsafe {
                        MessageBoxW(
                            Some(hwnd),
                            PCWSTR(wide.as_ptr()),
                            w!("Orbit updates"),
                            MB_OK | MB_ICONINFORMATION,
                        );
                    }
                } else {
                    notify_error(hwnd, &error);
                }
            }
            LRESULT(0)
        }
        OPEN_SETTINGS_MESSAGE => {
            if let Err(error) = settings_window::open() {
                notify_error(hwnd, &error);
            }
            LRESULT(0)
        }
        RELOAD_SETTINGS_MESSAGE => LRESULT(if reload_settings().is_ok() { 1 } else { -7 }),
        TRAY_MESSAGE if lparam.0 as u32 == WM_RBUTTONUP => {
            show_tray_menu(hwnd);
            LRESULT(0)
        }
        WM_HOTKEY if wparam.0 == HOTKEY_ID as usize => {
            if SESSION.with(|cell| cell.borrow().open || cell.borrow().trigger_pending) {
                return LRESULT(0);
            }
            let delay = SESSION.with(|cell| cell.borrow().settings.trigger_delay_ms);
            if delay == 0 {
                begin_radial(hwnd);
            } else {
                SESSION.with(|cell| {
                    let mut session = cell.borrow_mut();
                    session.trigger_pending = true;
                    session.trigger_started = Some(Instant::now());
                });
                unsafe {
                    let _ = SetTimer(Some(hwnd), TIMER_ID, 8, None);
                }
            }
            LRESULT(0)
        }
        WM_HOTKEY => {
            register_shortcut_action(hwnd, wparam.0 as i32);
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == TIMER_ID => {
            let (pending, opened, trigger, delay, start) = SESSION.with(|cell| {
                let session = cell.borrow();
                (
                    session.trigger_pending,
                    session.open,
                    session.settings.trigger,
                    session.settings.trigger_delay_ms,
                    session.trigger_started,
                )
            });
            if pending {
                if !trigger_held(trigger) {
                    SESSION.with(|cell| {
                        let mut session = cell.borrow_mut();
                        session.trigger_pending = false;
                        session.trigger_started = None;
                    });
                    unsafe {
                        let _ = KillTimer(Some(hwnd), TIMER_ID);
                    }
                } else if start
                    .is_some_and(|instant| instant.elapsed().as_millis() >= u128::from(delay))
                {
                    begin_radial(hwnd);
                }
                return LRESULT(0);
            }
            if !opened {
                let _ = unsafe { KillTimer(Some(hwnd), TIMER_ID) };
                return LRESULT(0);
            }
            if input_key_down(VK_ESCAPE.0) {
                finish_session(hwnd, false);
                return LRESULT(0);
            }
            let settings = SESSION.with(|cell| cell.borrow().settings.clone());
            if !settings.disable_cursor_interaction {
                let mut point = POINT::default();
                if unsafe { GetCursorPos(&mut point) }.is_ok() {
                    select_radial_cursor(hwnd, (point.x, point.y));
                }
            }
            if !trigger_held(settings.trigger) {
                finish_session(hwnd, true);
            }
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == UPDATE_TIMER_ID => {
            if SESSION.with(|cell| cell.borrow().settings.updates_enabled) {
                let _ = start_update_check(hwnd, false);
            }
            LRESULT(0)
        }
        DRAG_EVENT_MESSAGE => {
            let event = HOOK_EVENTS
                .lock()
                .ok()
                .and_then(|mut events| events.pop_front());
            if let Some(event) = event {
                if event.message == WM_LBUTTONUP && SESSION.with(|cell| cell.borrow().open) {
                    finish_session(hwnd, true);
                } else {
                    handle_drag_event(hwnd, event);
                }
            }
            LRESULT(0)
        }
        WHEEL_EVENT_MESSAGE => {
            handle_wheel_event(hwnd, wparam.0 as i16 as i32);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            if SESSION.with(|cell| cell.borrow().open) {
                finish_session(hwnd, true);
            }
            LRESULT(0)
        }
        WM_PAINT => {
            if SESSION.with(|cell| cell.borrow().preview == Some(hwnd)) {
                paint_preview(hwnd);
            } else {
                paint_radial(hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn handle_copydata(lparam: LPARAM) -> isize {
    if lparam.0 == 0 {
        return -1;
    }
    let packet = unsafe { &*(lparam.0 as *const COPYDATASTRUCT) };
    if packet.dwData != IPC_MAGIC
        || packet.cbData == 0
        || packet.cbData as usize > MAX_IPC_BYTES
        || packet.lpData.is_null()
    {
        return -1;
    }
    let bytes =
        unsafe { std::slice::from_raw_parts(packet.lpData.cast::<u8>(), packet.cbData as usize) };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return -1;
    };
    if value.get("version").and_then(serde_json::Value::as_u64) != Some(1) {
        return -1;
    }
    if value.get("reload").and_then(serde_json::Value::as_bool) == Some(true) {
        return if reload_settings().is_ok() { 1 } else { -7 };
    }
    let Ok(request) = serde_json::from_value::<DispatchRequest>(value) else {
        return -1;
    };
    if request.version != 1 || (request.target == 0 && request.action != Action::Unstash) {
        return -1;
    }
    let hwnd = HWND(request.target as *mut _);
    let settings = SESSION.with(|cell| cell.borrow().settings.clone());
    let result = execute_action(hwnd, request.action, &settings);
    match result {
        Ok(()) => 1,
        Err(error) => ipc_error_code(&error),
    }
}

fn ipc_error_code(error: &str) -> isize {
    if error.contains("excluded") {
        -2
    } else if error.contains("previous window position") {
        -3
    } else if error.contains("initial window position") {
        -4
    } else if error.contains("stashed window") {
        -5
    } else if error.contains("unsupported") {
        -6
    } else if error.contains("available") || error.contains("identity changed") {
        -1
    } else {
        -7
    }
}

fn select_radial_cursor(hwnd: HWND, cursor: (i32, i32)) {
    let moved = SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        if session.last_radial_cursor == Some(cursor) {
            return false;
        }
        session.last_radial_cursor = Some(cursor);
        true
    });
    if !moved {
        return;
    }
    let (origin, actions) = SESSION.with(|cell| {
        let session = cell.borrow();
        (session.origin, session.settings.radial_actions)
    });
    let dx = f64::from(cursor.0) - f64::from(origin.0);
    let dy = f64::from(cursor.1) - f64::from(origin.1);
    let sector = if dx * dx + dy * dy < 30.0 * 30.0 {
        None
    } else {
        Some(((dy.atan2(dx) / std::f64::consts::FRAC_PI_4).round() as i32).rem_euclid(8) as usize)
    };
    let next = sector.map(|index| actions[index]);
    let changed = SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        if session.selected_sector == sector && session.selected == next {
            return false;
        }
        session.selected_sector = sector;
        session.selected = next;
        true
    });
    if changed {
        redraw_radial(hwnd);
        update_preview();
    }
}

fn redraw_radial(host: HWND) {
    let (overlay, selected, sector, settings, target) = SESSION.with(|cell| {
        let session = cell.borrow();
        (
            session.overlay,
            session.selected,
            session.selected_sector,
            session.settings.clone(),
            session.target,
        )
    });
    if let Some(overlay) = overlay {
        let dpi = target
            .map(|window| unsafe { windows::Win32::UI::HiDpi::GetDpiForWindow(window) })
            .unwrap_or(96)
            .max(96);
        if let Err(error) =
            crate::radial::draw_with_settings_and_sector(overlay, selected, sector, &settings, dpi)
        {
            notify_error(host, &error);
        }
    }
}

fn handle_wheel_event(host: HWND, delta: i32) {
    let changed = SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        if !session.open || session.settings.disable_cursor_interaction {
            return None;
        }
        let reverse = session.settings.reverse_scroll;
        let step = if (delta > 0) ^ reverse {
            1isize
        } else {
            -1isize
        };
        let current = session.selected_sector.unwrap_or(0) as isize;
        let sector = (current + step).rem_euclid(8) as usize;
        session.selected_sector = Some(sector);
        session.selected = Some(session.settings.radial_actions[sector]);
        Some(())
    });
    if changed.is_some() {
        redraw_radial(host);
        update_preview();
    }
}

fn snap_action_at(point: POINT, threshold: i32) -> Option<Action> {
    let monitor =
        monitor_info(unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST) }).ok()?;
    let threshold = threshold.max(0);
    let left = point.x.saturating_sub(monitor.work.left).abs() <= threshold;
    let right = monitor.work.right.saturating_sub(point.x).abs() <= threshold;
    let top = point.y.saturating_sub(monitor.work.top).abs() <= threshold;
    let bottom = monitor.work.bottom.saturating_sub(point.y).abs() <= threshold;
    match (left, right, top, bottom) {
        (true, _, true, _) => Some(Action::TopLeft),
        (_, true, true, _) => Some(Action::TopRight),
        (true, _, _, true) => Some(Action::BottomLeft),
        (_, true, _, true) => Some(Action::BottomRight),
        (true, _, _, _) => Some(Action::LeftHalf),
        (_, true, _, _) => Some(Action::RightHalf),
        (_, _, true, _) => Some(Action::TopHalf),
        (_, _, _, true) => Some(Action::BottomHalf),
        _ => None,
    }
}

fn handle_drag_event(host: HWND, event: HookMouseEvent) {
    let point = event.point;
    match event.message {
        WM_LBUTTONDOWN => {
            if !SESSION.with(|cell| cell.borrow().settings.snap_on_drag) {
                return;
            }
            let under = unsafe { WindowFromPoint(point) };
            if under.0.is_null() {
                return;
            }
            let root = unsafe { GetAncestor(under, GA_ROOT) };
            let target = if root.0.is_null() { under } else { root };
            let settings = SESSION.with(|cell| cell.borrow().settings.clone());
            if ensure_target(target, &settings).is_err() {
                return;
            }
            let mut frame = RECT::default();
            if unsafe { GetWindowRect(target, &mut frame) }.is_err() {
                return;
            }
            SESSION.with(|cell| {
                cell.borrow_mut().drag = Some(DragState {
                    window: target,
                    start_cursor: point,
                    last_cursor: point,
                    start_frame: frame,
                    candidate: None,
                })
            });
        }
        WM_MOUSEMOVE => {
            let hovered = SESSION
                .with(|cell| {
                    cell.borrow()
                        .stash
                        .iter()
                        .rev()
                        .cloned()
                        .collect::<Vec<_>>()
                })
                .into_iter()
                .find(|entry| {
                    recovery_entry_is_current(entry)
                        && rect_from_window(entry.window).is_ok_and(|r| {
                            point.x >= r.left
                                && point.x < r.right
                                && point.y >= r.top
                                && point.y < r.bottom
                        })
                });
            if let Some(entry) = hovered {
                let result = restore_placement(entry.window, &entry.placement)
                    .and_then(|()| clear_recovery(entry.window, false));
                if let Err(error) = result {
                    notify_error(host, &error);
                }
            }
            let Some((window, start, frame, threshold)) = SESSION.with(|cell| {
                let session = cell.borrow();
                let drag = session.drag.as_ref()?;
                Some((
                    drag.window,
                    drag.start_cursor,
                    drag.start_frame,
                    session.settings.snap_threshold,
                ))
            }) else {
                return;
            };
            let mut current = RECT::default();
            let moved = unsafe { GetWindowRect(window, &mut current) }.is_ok()
                && ((current.left - frame.left).abs() >= 4 || (current.top - frame.top).abs() >= 4)
                && ((point.x - start.x).abs() >= 4 || (point.y - start.y).abs() >= 4);
            let candidate = if moved {
                snap_action_at(point, threshold)
            } else {
                None
            };
            SESSION.with(|cell| {
                if let Some(drag) = cell.borrow_mut().drag.as_mut() {
                    drag.last_cursor = point;
                    drag.candidate = candidate;
                }
            });
            update_preview();
        }
        WM_LBUTTONUP => {
            let (drag, settings) = SESSION.with(|cell| {
                let mut session = cell.borrow_mut();
                (session.drag.take(), session.settings.clone())
            });
            update_preview();
            if let Some(drag) = drag
                && let Some(action) = drag.candidate
                && let Err(error) = execute_action(drag.window, action, &settings)
            {
                notify_error(host, &error);
            }
        }
        _ => {}
    }
}

unsafe extern "system" fn mouse_hook_proc(code: i32, message: WPARAM, details: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && details.0 != 0 {
        let data = unsafe { &*(details.0 as *const MSLLHOOKSTRUCT) };
        let event = message.0 as u32;
        let (wants_drag_event, wants_wheel, wants_radial_release) = SESSION.with(|cell| {
            let session = cell.borrow();
            let wants_drag = session.settings.snap_on_drag;
            (
                wants_drag && matches!(event, WM_LBUTTONDOWN | WM_LBUTTONUP)
                    || event == WM_MOUSEMOVE
                        && (wants_drag && session.drag.is_some() || !session.stash.is_empty()),
                event == WM_MOUSEWHEEL
                    && session.open
                    && !session.settings.disable_cursor_interaction,
                event == WM_LBUTTONUP && session.open,
            )
        });
        if wants_drag_event || wants_wheel || wants_radial_release {
            let host = HOOK_HOST.load(Ordering::Relaxed);
            if host != 0 {
                if wants_wheel {
                    let delta = ((data.mouseData >> 16) as u16 as i16) as usize;
                    let _ = unsafe {
                        PostMessageW(
                            Some(HWND(host as *mut _)),
                            WHEEL_EVENT_MESSAGE,
                            WPARAM(delta),
                            LPARAM(0),
                        )
                    };
                } else {
                    if let Ok(mut events) = HOOK_EVENTS.lock() {
                        let hook_event = HookMouseEvent {
                            message: event,
                            point: data.pt,
                        };
                        if events.len() >= 128 {
                            if let Some(index) = events
                                .iter()
                                .position(|queued| queued.message == WM_MOUSEMOVE)
                            {
                                events.remove(index);
                            } else if event == WM_MOUSEMOVE {
                                return unsafe { CallNextHookEx(None, code, message, details) };
                            }
                        }
                        events.push_back(hook_event);
                        if unsafe {
                            PostMessageW(
                                Some(HWND(host as *mut _)),
                                DRAG_EVENT_MESSAGE,
                                WPARAM(0),
                                LPARAM(0),
                            )
                        }
                        .is_err()
                        {
                            events.pop_back();
                        }
                    }
                }
            }
        }
    }
    unsafe { CallNextHookEx(None, code, message, details) }
}

fn notify_error(host: HWND, message: &str) {
    let mut data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: host,
        uID: 1,
        uFlags: NIF_INFO,
        dwInfoFlags: NIIF_ERROR,
        ..Default::default()
    };
    let title: Vec<u16> = "Orbit".encode_utf16().chain(Some(0)).collect();
    let info: Vec<u16> = message
        .chars()
        .take(255)
        .collect::<String>()
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let title_len = title.len().min(data.szInfoTitle.len() - 1);
    let info_len = info.len().min(data.szInfo.len() - 1);
    data.szInfoTitle[..title_len].copy_from_slice(&title[..title_len]);
    data.szInfo[..info_len].copy_from_slice(&info[..info_len]);
    unsafe {
        let _ = Shell_NotifyIconW(windows::Win32::UI::Shell::NIM_MODIFY, &data);
    }
}

fn start_update_check(host: HWND, manual: bool) -> Result<(), String> {
    if UPDATE_DOWNLOADING.load(Ordering::Acquire)
        || UPDATE_PROMPT_PENDING.load(Ordering::Acquire)
        || UPDATE_CHECKING.swap(true, Ordering::AcqRel)
    {
        return if manual {
            Err("An update check or download is already in progress".into())
        } else {
            Ok(())
        };
    }
    let address = host.0 as usize;
    std::thread::spawn(move || {
        match update::check() {
            Ok(Some(manifest)) => {
                if let Ok(mut slot) = AVAILABLE_UPDATE.lock() {
                    *slot = Some(manifest);
                }
                let host = HWND(address as *mut _);
                UPDATE_PROMPT_PENDING.store(true, Ordering::Release);
                if unsafe {
                    PostMessageW(Some(host), UPDATE_AVAILABLE_MESSAGE, WPARAM(0), LPARAM(0))
                }
                .is_err()
                {
                    UPDATE_PROMPT_PENDING.store(false, Ordering::Release);
                }
            }
            Ok(None) if manual => {
                if let Ok(mut slot) = UPDATE_ERROR.lock() {
                    *slot = Some("No newer stable release is available".into());
                }
                let _ = unsafe {
                    PostMessageW(
                        Some(HWND(address as *mut _)),
                        UPDATE_FAILED_MESSAGE,
                        WPARAM(1),
                        LPARAM(0),
                    )
                };
            }
            Ok(None) => {}
            Err(error) if manual => post_update_status(HWND(address as *mut _), error),
            Err(error) => eprintln!("Orbit update check: {error}"),
        }
        UPDATE_CHECKING.store(false, Ordering::Release);
    });
    Ok(())
}

fn start_update_download(host: HWND, manifest: Manifest) {
    if UPDATE_DOWNLOADING.swap(true, Ordering::AcqRel) {
        return;
    }
    let address = host.0 as usize;
    std::thread::spawn(move || {
        let host = HWND(address as *mut _);
        match update::download(&manifest) {
            Ok(path) => {
                if let Ok(mut slot) = DOWNLOADED_UPDATE.lock() {
                    *slot = Some((path, manifest));
                }
                if unsafe { PostMessageW(Some(host), UPDATE_READY_MESSAGE, WPARAM(0), LPARAM(0)) }
                    .is_err()
                {
                    UPDATE_DOWNLOADING.store(false, Ordering::Release);
                }
            }
            Err(error) => {
                UPDATE_DOWNLOADING.store(false, Ordering::Release);
                post_update_status(host, error);
            }
        }
    });
}

fn post_update_status(host: HWND, message: String) {
    if let Ok(mut slot) = UPDATE_ERROR.lock() {
        *slot = Some(message);
    }
    let _ = unsafe { PostMessageW(Some(host), UPDATE_FAILED_MESSAGE, WPARAM(0), LPARAM(0)) };
}

fn show_tray_menu(hwnd: HWND) {
    let Ok(menu) = (unsafe { CreatePopupMenu() }) else {
        return;
    };
    unsafe {
        let _ = AppendMenuW(menu, MF_STRING, 3, w!("Settings..."));
        let _ = AppendMenuW(menu, MF_STRING, 4, w!("Undo last move"));
        let _ = AppendMenuW(menu, MF_STRING, 5, w!("Restore last hidden window"));
        let _ = AppendMenuW(menu, MF_STRING, 6, w!("Restore last stashed window"));
        let _ = AppendMenuW(menu, MF_STRING, 1, w!("About Orbit"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(menu, MF_STRING, 2, w!("Quit Orbit"));
    }
    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point) }.is_ok() {
        let _ = unsafe { SetForegroundWindow(hwnd) };
        let command = unsafe {
            TrackPopupMenu(
                menu,
                TPM_RIGHTBUTTON | TPM_RETURNCMD,
                point.x,
                point.y,
                None,
                hwnd,
                None,
            )
        }
        .0;
        match command {
            4 => {
                let (target, settings) = SESSION.with(|cell| {
                    let mut session = cell.borrow_mut();
                    (session.history.last_window(), session.settings.clone())
                });
                let result = target
                    .ok_or_else(|| "There is no previous window position to undo".to_string())
                    .and_then(|target| execute_action(target, Action::Undo, &settings));
                if let Err(error) = result {
                    notify_error(hwnd, &error);
                }
            }
            5 => {
                let target =
                    SESSION.with(|cell| cell.borrow().hidden.last().map(|entry| entry.window));
                let result = target
                    .ok_or_else(|| "There is no hidden window to restore".to_string())
                    .and_then(restore_hidden_window);
                if let Err(error) = result {
                    notify_error(hwnd, &error);
                }
            }
            6 => {
                let settings = SESSION.with(|cell| cell.borrow().settings.clone());
                if let Err(error) = unstash(&settings) {
                    notify_error(hwnd, &error);
                }
            }
            3 => {
                if let Err(error) = settings_window::open() {
                    notify_error(hwnd, &error);
                }
            }
            1 => {
                let about: Vec<u16> = format!(
                    "Orbit {}\nGPL-3.0-only\nAdapted from Loop by Kai Azim and contributors.",
                    env!("CARGO_PKG_VERSION")
                )
                .encode_utf16()
                .chain(Some(0))
                .collect();
                unsafe {
                    MessageBoxW(Some(hwnd), PCWSTR(about.as_ptr()), w!("About Orbit"), MB_OK);
                }
            }
            2 => unsafe {
                let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
            },
            _ => {}
        }
    }
    let _ = unsafe { DestroyMenu(menu) };
}

pub fn reload_settings() -> Result<(), String> {
    let next = Settings::load()?;
    let (host, previous) = SESSION.with(|cell| {
        let session = cell.borrow();
        (session.host, session.settings.clone())
    });
    let Some(host) = host else {
        return Ok(());
    };
    finish_session(host, false);
    SESSION.with(|cell| cell.borrow_mut().drag = None);
    unregister_hotkeys(host);
    let shortcuts = match register_hotkeys(host, &next) {
        Ok(shortcuts) => shortcuts,
        Err(error) => {
            let rollback = register_hotkeys(host, &previous);
            return match rollback {
                Ok(shortcuts) => {
                    SESSION.with(|cell| cell.borrow_mut().shortcuts = shortcuts);
                    Err(error)
                }
                Err(rollback) => Err(format!(
                    "{error}. Could not restore previous shortcuts: {rollback}"
                )),
            };
        }
    };
    SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        session.settings = next.clone();
        session.shortcuts = shortcuts;
    });
    unsafe {
        let _ = KillTimer(Some(host), UPDATE_TIMER_ID);
    }
    if next.updates_enabled && update::configured().is_some() {
        unsafe {
            SetTimer(Some(host), UPDATE_TIMER_ID, 6 * 60 * 60 * 1000, None);
        }
    }
    update_preview();
    Ok(())
}

fn finish_session(host: HWND, commit: bool) {
    let state = SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        session.trigger_pending = false;
        session.trigger_started = None;
        if !session.open {
            return None;
        }
        session.open = false;
        session.selected_sector = None;
        Some((
            session.overlay,
            session.target.take(),
            session.selected.take(),
            session.settings.clone(),
        ))
    });
    let _ = unsafe { KillTimer(Some(host), TIMER_ID) };
    let Some((overlay, target, selected, settings)) = state else {
        return;
    };
    if let Some(overlay) = overlay {
        unsafe {
            let _ = ShowWindow(overlay, SW_HIDE);
        }
    }
    update_preview();
    if !commit {
        return;
    }
    if let (Some(target), Some(action)) = (target, selected)
        && let Err(error) = execute_action(target, action, &settings)
    {
        notify_error(host, &error);
    }
}

fn update_preview() {
    let (preview, selection, settings) = SESSION.with(|cell| {
        let session = cell.borrow();
        let selection = if session.open {
            session.target.zip(session.selected)
        } else {
            session
                .drag
                .as_ref()
                .and_then(|drag| drag.candidate.map(|action| (drag.window, action)))
        };
        (session.preview, selection, session.settings.clone())
    });
    let frame = selection
        .filter(|(_, action)| settings.preview_visible && action_has_preview(*action))
        .and_then(|(target, action)| target_frame(target, action, &settings).ok())
        .map(|frame| frame.inset(settings.preview_padding));
    if let Some(preview) = preview {
        if let Some(frame) = frame {
            unsafe {
                let _ = SetLayeredWindowAttributes(
                    preview,
                    COLORREF(0),
                    settings.preview_opacity,
                    LWA_ALPHA,
                );
                let _ = SetWindowPos(
                    preview,
                    None,
                    frame.left,
                    frame.top,
                    frame.width(),
                    frame.height(),
                    SWP_NOACTIVATE | SWP_NOZORDER,
                );
                let radius = (settings.preview_corner_radius as i32)
                    .min(frame.width().min(frame.height()) / 2);
                let region = windows::Win32::Graphics::Gdi::CreateRoundRectRgn(
                    0,
                    0,
                    frame.width() + 1,
                    frame.height() + 1,
                    radius * 2,
                    radius * 2,
                );
                if !region.0.is_null()
                    && windows::Win32::Graphics::Gdi::SetWindowRgn(preview, Some(region), true) == 0
                {
                    let _ = DeleteObject(HGDIOBJ(region.0));
                }
                let _ = InvalidateRect(Some(preview), None, false);
                let _ = ShowWindow(preview, SW_SHOWNOACTIVATE);
            }
        } else {
            unsafe {
                let _ = ShowWindow(preview, SW_HIDE);
            }
        }
    }
}

fn action_has_preview(action: Action) -> bool {
    !matches!(
        action,
        Action::NoAction
            | Action::Hide
            | Action::Minimize
            | Action::MinimizeOthers
            | Action::Restore
            | Action::Undo
            | Action::InitialFrame
            | Action::NextMonitor
            | Action::PreviousMonitor
            | Action::MoveToMonitorLeft
            | Action::MoveToMonitorRight
            | Action::MoveToMonitorUp
            | Action::MoveToMonitorDown
            | Action::FocusLeft
            | Action::FocusRight
            | Action::FocusUp
            | Action::FocusDown
            | Action::FocusNextInStack
            | Action::Stash
            | Action::StashLeft
            | Action::StashRight
            | Action::StashUp
            | Action::StashDown
            | Action::Unstash
    )
}

fn paint_preview(hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    let dc = unsafe { BeginPaint(hwnd, &mut ps) };
    let mut area = RECT::default();
    let color = SESSION.with(|cell| cell.borrow().settings.accent_color);
    let colorref = ((color >> 16) & 0xff) | (color & 0xff00) | ((color & 0xff) << 16);
    let brush = unsafe { CreateSolidBrush(COLORREF(colorref)) };
    unsafe {
        if GetClientRect(hwnd, &mut area).is_ok() {
            FillRect(dc, &area, brush);
        }
        let _ = DeleteObject(HGDIOBJ(brush.0));
        let _ = EndPaint(hwnd, &ps);
    }
}

fn paint_radial(hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    unsafe {
        BeginPaint(hwnd, &mut ps);
    }
    let host = SESSION.with(|cell| cell.borrow().host);
    if let Some(host) = host {
        redraw_radial(host);
    }
    unsafe {
        let _ = EndPaint(hwnd, &ps);
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    struct WindowGuard(HWND);
    impl Drop for WindowGuard {
        fn drop(&mut self) {
            let _ = unsafe { DestroyWindow(self.0) };
        }
    }

    #[test]
    #[ignore = "opens a native window; requires an explicitly approved interactive desktop"]
    fn moves_a_real_owned_window_to_work_area_half() {
        let module = unsafe { GetModuleHandleW(None) }.unwrap();
        let window = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!("Orbit API test"),
                WS_POPUP | windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE,
                40,
                40,
                300,
                220,
                None,
                None,
                Some(HINSTANCE(module.0)),
                None,
            )
        }
        .unwrap();
        let _guard = WindowGuard(window);
        let monitor = unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) };
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        assert!(unsafe { GetMonitorInfoW(monitor, &mut info).as_bool() });
        apply(window, Action::LeftHalf, 0).unwrap();
        let mut actual = RECT::default();
        unsafe { GetWindowRect(window, &mut actual) }.unwrap();
        let work = info.rcWork;
        assert_eq!(actual.left, work.left);
        assert_eq!(actual.right, work.left + (work.right - work.left) / 2);
        assert_eq!(actual.top, work.top);
        assert_eq!(actual.bottom, work.bottom);
    }

    #[test]
    #[ignore = "opens a native window; requires an explicitly approved interactive desktop"]
    fn history_restores_real_window_and_rejects_destroyed_target() {
        let window = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!("Orbit undo test"),
                WS_POPUP | windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE,
                80,
                90,
                310,
                230,
                None,
                None,
                None,
                None,
            )
        }
        .unwrap();
        let guard = WindowGuard(window);
        let before = history::placement(window).unwrap();
        let original = before.rcNormalPosition;
        let mut history = History::default();
        apply(window, Action::RightHalf, 0).unwrap();
        history.record(window, before).unwrap();
        history.undo(Some(window)).unwrap();
        let restored = history::placement(window).unwrap().rcNormalPosition;
        assert_eq!(
            (restored.left, restored.top, restored.right, restored.bottom),
            (original.left, original.top, original.right, original.bottom)
        );
        history.record(window, before).unwrap();
        drop(guard);
        assert!(history.undo(None).is_err());
    }
}
