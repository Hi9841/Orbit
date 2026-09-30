use crate::history::{self, History};
use crate::settings_window;
use orbit::geometry::{Action, Rect};
use orbit::settings::{Hotkey, PreviewStart, Settings, TriggerSide};
use orbit::update::{self, Manifest};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use windows::Win32::Foundation::{
    COLORREF, ERROR_ALREADY_EXISTS, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows::Win32::Graphics::Dwm::{
    DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetColorizationColor, DwmGetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BeginPaint, BitBlt, COLORONCOLOR, CreateCompatibleDC,
    CreateDCW, CreateDIBSection, CreateSolidBrush, DIB_RGB_COLORS, DeleteDC, DeleteObject,
    EndPaint, EnumDisplayMonitors, FillRect, GetDC, GetDeviceCaps, GetMonitorInfoW, HGDIOBJ,
    HMONITOR, HORZSIZE, InvalidateRect, MONITOR_DEFAULTTONEAREST, MONITORINFO, MONITORINFOEXW,
    MonitorFromPoint, MonitorFromWindow, PAINTSTRUCT, ReleaseDC, SRCCOPY, SelectObject,
    SetDIBitsToDevice, SetStretchBltMode, StretchDIBits, UpdateWindow, VERTSIZE,
};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::System::Threading::{
    CreateMutexW, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetDoubleClickTime, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, VK_DOWN, VK_ESCAPE,
    VK_LEFT, VK_RIGHT, VK_SHIFT, VK_UP,
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
    GetAncestor, GetClassNameW, GetClientRect, GetCursorPos, GetForegroundWindow, GetMessageW,
    GetPropW, GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, HC_ACTION, HICON,
    HTTRANSPARENT, HWND_TOPMOST, IDC_ARROW, IDI_APPLICATION, IDYES, IsHungAppWindow, IsIconic,
    IsWindow, IsWindowVisible, IsZoomed, KBDLLHOOKSTRUCT, KillTimer, LoadCursorW, LoadIconW,
    MB_ICONINFORMATION, MB_OK, MB_YESNO, MF_SEPARATOR, MF_STRING, MSG, MSLLHOOKSTRUCT, MessageBoxW,
    PostMessageW, PostQuitMessage, RegisterClassW, RemovePropW, SHOW_WINDOW_CMD, SMTO_ABORTIFHUNG,
    SMTO_BLOCK, SW_HIDE, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SendMessageTimeoutW, SetForegroundWindow,
    SetLayeredWindowAttributes, SetPropW, SetTimer, SetWindowPlacement, SetWindowPos,
    SetWindowsHookExW, ShowWindow, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu,
    TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL, WH_MOUSE_LL, WINDOWPLACEMENT, WM_APP,
    WM_CLOSE, WM_COPYDATA, WM_DESTROY, WM_HOTKEY, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCHITTEST,
    WM_PAINT, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TIMER, WNDCLASSW, WS_CAPTION,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_OVERLAPPED, WS_POPUP,
    WS_THICKFRAME, WindowFromPoint,
};
use windows::Win32::UI::WindowsAndMessaging::{
    LWA_ALPHA, LWA_COLORKEY, SPI_GETCLIENTAREAANIMATION, SPI_GETUIEFFECTS,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW, WS_EX_TRANSPARENT,
};
use windows::core::BOOL;
use windows::core::{PCWSTR, w};

const HOTKEY_ID: i32 = 1;
const SHORTCUT_HOTKEY_BASE: i32 = 100;
const TIMER_ID: usize = 1;
const UPDATE_TIMER_ID: usize = 2;
const ANIMATION_TIMER_ID: usize = 3;
const PREVIEW_ANIMATION_TIMER_ID: usize = 4;
const TRAY_MESSAGE: u32 = WM_APP + 1;
const OPEN_SETTINGS_MESSAGE: u32 = WM_APP + 2;
const UPDATE_AVAILABLE_MESSAGE: u32 = WM_APP + 3;
const UPDATE_READY_MESSAGE: u32 = WM_APP + 4;
const UPDATE_FAILED_MESSAGE: u32 = WM_APP + 5;
const RELOAD_SETTINGS_MESSAGE: u32 = WM_APP + 6;
const DRAG_EVENT_MESSAGE: u32 = WM_APP + 7;
const WHEEL_EVENT_MESSAGE: u32 = WM_APP + 8;
const TRIGGER_EVAL_MESSAGE: u32 = WM_APP + 9;
const IPC_MAGIC: usize = 0x4f52_4254;
const IPC_TIMEOUT_MS: u32 = 5000;
const MAX_IPC_BYTES: usize = 4096;
const RECOVERY_PROPERTY: windows::core::PCWSTR = w!("Orbit.Recovery.6C96F66A");
// COLORREF stores red in the low byte; a 32-bit BI_RGB pixel stores it in the high color byte.
const TRANSPARENT_COLOR_KEY: COLORREF = COLORREF(0x0003_0201);
const TRANSPARENT_DIB_PIXEL: u32 = 0x0001_0203;
const RADIAL_WINDOW_OPACITY: u8 = 255;

// Layered popups stay on the color-key path (WM_PAINT + SetDIBitsToDevice).
// Measured on this machine (DPI 96): UpdateLayeredWindow per-pixel alpha left a
// monitor-sized window "visible" while GetPixel read the window underneath, and
// a 180x120 magenta probe found no 0x00ff00ff pixels. A color-key probe passed.
// Do not call UpdateLayeredWindow after SetLayeredWindowAttributes: the combo
// disables per-pixel alpha until the layered style is reset.
fn enable_color_key(window: HWND, opacity: u8) -> Result<(), String> {
    unsafe {
        SetLayeredWindowAttributes(
            window,
            TRANSPARENT_COLOR_KEY,
            opacity,
            LWA_COLORKEY | LWA_ALPHA,
        )
    }
    .map_err(|error| format!("cannot enable popup transparency: {error}"))
}

pub fn system_accent_rgb() -> Option<u32> {
    let mut color = 0;
    let mut opaque = BOOL(0);
    unsafe { DwmGetColorizationColor(&mut color, &mut opaque) }
        .ok()
        .map(|()| color & 0x00ff_ffff)
}
static HOOK_HOST: AtomicUsize = AtomicUsize::new(0);
static TASKBAR_CREATED_MESSAGE: AtomicU32 = AtomicU32::new(0);
static MIDDLE_CLICK_ENABLED: AtomicBool = AtomicBool::new(false);
static MIDDLE_CLICK_HELD: AtomicBool = AtomicBool::new(false);
static TRIGGER_KEY: AtomicU32 = AtomicU32::new(0);
/// Physical state of the non-modifier trigger key, observed by the low-level hook.
/// RegisterHotKey can report the chord before GetAsyncKeyState sees the key.
static TRIGGER_KEY_HELD: AtomicBool = AtomicBool::new(false);
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
    trigger_kind: TriggerKind,
    trigger_wait_double_tap: bool,
    last_trigger_key_release: Option<Instant>,
    last_middle_release: Option<Instant>,
    tray_icon: Option<HICON>,
    tray_installed: bool,
    animations: Vec<WindowAnimation>,
    animation_versions: Vec<(isize, u64)>,
    animation_tick_active: bool,
    preview_bitmap_cache: Option<(PreviewStyleKey, PresentedBitmap, bool)>,
    preview_layer_opacity: Option<u8>,
    preview_current_frame: Option<Rect>,
    preview_animation: Option<PreviewAnimation>,
    radial_bitmap: Option<PresentedBitmap>,
    radial_cache_key: Option<RadialCacheKey>,
    radial_cache: Option<Vec<Option<PresentedBitmap>>>,
    last_preview_full_build: Option<Instant>,
    arrow_state: u8,
    drag: Option<DragState>,
}

#[derive(Clone)]
struct ShortcutCycle {
    ids: Vec<i32>,
    actions: Vec<Action>,
    progress: Vec<ShortcutProgress>,
}

#[derive(Clone, Copy)]
struct ShortcutProgress {
    window: HWND,
    process_id: u32,
    token: usize,
    next: usize,
    last_press: Instant,
}

fn shortcut_progress_matches(
    progress: &ShortcutProgress,
    window: HWND,
    process_id: u32,
    token: usize,
    now: Instant,
    timeout: std::time::Duration,
) -> bool {
    progress.window == window
        && progress.process_id == process_id
        && progress.token == token
        && now.saturating_duration_since(progress.last_press) <= timeout
}

fn refresh_cycle_identity(
    cycles: &mut [ShortcutCycle],
    window: HWND,
    process_id: u32,
    token: usize,
) -> usize {
    let mut refreshed = 0;
    for cycle in cycles {
        for progress in &mut cycle.progress {
            if progress.window == window && progress.process_id == process_id {
                progress.token = token;
                refreshed += 1;
            }
        }
    }
    refreshed
}

fn cycle_action_index(next: usize, action_count: usize, reverse: bool) -> usize {
    if action_count == 0 {
        return 0;
    }
    if reverse {
        (next % action_count + action_count - 1) % action_count
    } else {
        next % action_count
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum TriggerKind {
    #[default]
    Keyboard,
    MiddleMouse,
}

#[derive(Clone)]
struct StashedWindow {
    window: HWND,
    placement: windows::Win32::UI::WindowsAndMessaging::WINDOWPLACEMENT,
    process_id: u32,
    token: usize,
    hidden: bool,
    original_frame: Rect,
}

struct WindowAnimation {
    window: HWND,
    process_id: u32,
    history_token: usize,
    recovery_token: usize,
    generation: u64,
    from: Rect,
    to: Rect,
    started: Instant,
    duration: std::time::Duration,
    completion: AnimationCompletion,
}

#[derive(Clone)]
struct PresentedBitmap {
    width: i32,
    height: i32,
    pixels: Arc<[u32]>,
}

impl PresentedBitmap {
    fn from_premultiplied(width: i32, height: i32, pixels: &[u32]) -> Self {
        Self {
            width,
            height,
            pixels: pixels.iter().copied().map(color_key_pixel).collect(),
        }
    }
}

#[derive(Clone, Copy)]
struct PreviewAnimation {
    from: Rect,
    to: Rect,
    started: Instant,
    duration: std::time::Duration,
}

impl PreviewAnimation {
    fn frame_at(self, now: Instant) -> (Rect, bool) {
        let progress = (now.saturating_duration_since(self.started).as_secs_f64()
            / self.duration.as_secs_f64())
        .clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - progress).powi(3);
        (interpolate_rect(self.from, self.to, eased), progress >= 1.0)
    }
}

enum AnimationCompletion {
    None,
    Unstash { entry: StashedWindow, focus: bool },
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct PreviewStyleKey {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    dpi: u32,
    accent: u32,
    gradient: u32,
    border: u32,
    radius: u32,
    opacity: u8,
    use_gradient: bool,
    action: Action,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct RadialCacheKey {
    size: u32,
    thickness: u32,
    radius: u32,
    dpi: u32,
    accent: u32,
    gradient: u32,
    use_gradient: bool,
    use_system_accent: bool,
}

/// Full preview rebuilds (screen capture + CPU blur + two presents) block the
/// UI thread for ~17 ms in release (~150 ms in debug) per sector change.
/// Rapid cursor moves must coalesce into one rebuild or the ring visibly jumps.
const PREVIEW_FULL_BUILD_DEBOUNCE_MS: u64 = 40;

fn preview_full_build_due(last: Option<Instant>, now: Instant) -> bool {
    last.is_none_or(|built| {
        now.saturating_duration_since(built)
            >= std::time::Duration::from_millis(PREVIEW_FULL_BUILD_DEBOUNCE_MS)
    })
}

fn radial_cache_key_for(settings: &Settings, dpi: u32) -> RadialCacheKey {
    let accent = settings
        .use_system_accent
        .then(system_accent_rgb)
        .flatten()
        .unwrap_or(settings.accent_color);
    RadialCacheKey {
        size: settings.radial_size,
        thickness: settings.radial_thickness,
        radius: settings.radial_corner_radius,
        dpi,
        accent,
        gradient: settings.gradient_color,
        use_gradient: settings.use_gradient,
        use_system_accent: settings.use_system_accent,
    }
}

/// Render every ring state once so the hotkey path is a cache hit plus one
/// small blit. Pure CPU work, safe to run at startup and on settings reload.
fn prewarm_radial_cache(settings: &Settings, dpi: u32) {
    let key = radial_cache_key_for(settings, dpi);
    let already_warm = SESSION.with(|cell| {
        let session = cell.borrow();
        session.radial_cache_key == Some(key)
            && session
                .radial_cache
                .as_ref()
                .is_some_and(|cache| cache.iter().all(|slot| slot.is_some()))
    });
    if already_warm {
        return;
    }
    let mut slots: Vec<Option<PresentedBitmap>> = vec![None; 9];
    for (slot, entry) in slots.iter_mut().enumerate() {
        let (selected, sector) = if slot == 8 {
            (None, None)
        } else {
            (
                Some(settings.radial_actions[slot]),
                Some(slot),
            )
        };
        if let Ok(bitmap) = crate::radial::compose_radial(selected, sector, settings, dpi) {
            *entry = Some(PresentedBitmap::from_premultiplied(
                bitmap.size,
                bitmap.size,
                &bitmap.pixels,
            ));
        }
    }
    SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        // Keep a freshly painted frame if settings changed mid-session.
        if session.radial_cache_key != Some(key) {
            session.radial_cache_key = Some(key);
            session.radial_cache = Some(slots);
        }
    });
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
    start_visible: Rect,
    restore_frame: bool,
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
    ensure_target_identity(hwnd, settings)?;
    ensure_fullscreen_policy(hwnd, settings)
}

fn ensure_fullscreen_policy(hwnd: HWND, settings: &Settings) -> Result<(), String> {
    if settings.ignore_fullscreen {
        let monitor = monitor_info(unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) })?;
        if is_fullscreen(hwnd, monitor) {
            return Err("full-screen windows are excluded in Orbit settings".into());
        }
    }
    Ok(())
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
    physical_inches: Option<f64>,
}

fn monitor_info(handle: HMONITOR) -> Result<Monitor, String> {
    if handle.0.is_null() {
        return Err("cannot find target monitor".into());
    }
    let mut info = MONITORINFOEXW {
        monitorInfo: MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(handle, &mut info.monitorInfo).as_bool() } {
        return Err(format!(
            "cannot read monitor work area: {}",
            windows::core::Error::from_thread()
        ));
    }
    let device_end = info
        .szDevice
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(info.szDevice.len());
    let display = unsafe {
        CreateDCW(
            w!("DISPLAY"),
            PCWSTR(info.szDevice.as_ptr()),
            PCWSTR::null(),
            None,
        )
    };
    let physical_inches = if display.0.is_null() {
        None
    } else {
        let width_mm = unsafe { GetDeviceCaps(Some(display), HORZSIZE) };
        let height_mm = unsafe { GetDeviceCaps(Some(display), VERTSIZE) };
        let _ = unsafe { DeleteDC(display) };
        (width_mm > 0 && height_mm > 0)
            .then(|| f64::from(width_mm).hypot(f64::from(height_mm)) / 25.4)
    };
    let _ = device_end;
    Ok(Monitor {
        handle,
        work: Rect {
            left: info.monitorInfo.rcWork.left,
            top: info.monitorInfo.rcWork.top,
            right: info.monitorInfo.rcWork.right,
            bottom: info.monitorInfo.rcWork.bottom,
        },
        full: Rect {
            left: info.monitorInfo.rcMonitor.left,
            top: info.monitorInfo.rcMonitor.top,
            right: info.monitorInfo.rcMonitor.right,
            bottom: info.monitorInfo.rcMonitor.bottom,
        },
        physical_inches,
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
    let work = effective_work_area(monitor, settings);
    if action == Action::Fullscreen {
        return Ok(monitor.full);
    }
    if action == Action::MacOSCenter {
        let width = current.width().min(work.width());
        let height = current.height().min(work.height());
        let left = work.left + (work.width() - width) / 2;
        let top = work.top + (work.height() - height) / 4;
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
            orbit::available_space::largest_frame(work, current, &obstacles)
                .inset(settings.padding),
        );
    }
    if let Action::Custom(index) = action {
        let frame = settings
            .custom_frames
            .get(usize::from(index))
            .ok_or_else(|| format!("custom frame {index} does not exist"))?;
        let x = work.width();
        let y = work.height();
        return Ok(Rect {
            left: work.left + (f64::from(x) * frame.x).round() as i32,
            top: work.top + (f64::from(y) * frame.y).round() as i32,
            right: work.left + (f64::from(x) * (frame.x + frame.width)).round() as i32,
            bottom: work.top + (f64::from(y) * (frame.y + frame.height)).round() as i32,
        }
        .inset(settings.padding));
    }
    Ok(action.frame_with_increment(work, current, settings.padding, settings.size_increment))
}

fn effective_work_area(monitor: Monitor, settings: &Settings) -> Rect {
    let Some(edge) = settings.edge_padding else {
        return monitor.work;
    };
    let minimum = settings.padding_minimum_screen_inches;
    if minimum > 0.0 && !monitor.physical_inches.is_some_and(|size| size > minimum) {
        return monitor.work;
    }
    monitor
        .work
        .inset_edges(edge.top, edge.right, edge.bottom, edge.left)
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
    position_visible_frame(hwnd, target, extra_flags)?;
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

fn position_visible_frame(
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
    Ok(())
}

fn animations_allowed(settings: &Settings) -> bool {
    if settings.animation_duration_ms == 0 {
        return false;
    }
    for action in [SPI_GETCLIENTAREAANIMATION, SPI_GETUIEFFECTS] {
        let mut enabled = BOOL(1);
        if unsafe {
            SystemParametersInfoW(
                action,
                0,
                Some((&mut enabled as *mut BOOL).cast()),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )
        }
        .is_ok()
            && !enabled.as_bool()
        {
            return false;
        }
    }
    if !settings.ignore_low_power_mode {
        let mut status = SYSTEM_POWER_STATUS::default();
        if unsafe { GetSystemPowerStatus(&mut status) }.is_ok() && status.SystemStatusFlag == 1 {
            return false;
        }
    }
    true
}

fn bump_animation_generation(session: &mut Session, hwnd: HWND) -> u64 {
    let key = hwnd.0 as isize;
    if let Some((_, generation)) = session
        .animation_versions
        .iter_mut()
        .find(|(window, _)| *window == key)
    {
        *generation = generation.wrapping_add(1).max(1);
        *generation
    } else {
        session.animation_versions.push((key, 1));
        1
    }
}

fn animation_is_current(hwnd: HWND, generation: u64) -> bool {
    SESSION.with(|cell| {
        cell.borrow()
            .animation_versions
            .iter()
            .any(|(window, current)| *window == hwnd.0 as isize && *current == generation)
    })
}

fn cancel_window_animation(hwnd: HWND) {
    if hwnd.0.is_null() {
        return;
    }
    let host = SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        let needs_invalidation = session.animation_tick_active
            || session
                .animations
                .iter()
                .any(|animation| animation.window == hwnd);
        session
            .animations
            .retain(|animation| animation.window != hwnd);
        if needs_invalidation {
            bump_animation_generation(&mut session, hwnd);
        }
        session.host
    });
    if let Some(host) = host {
        let idle = SESSION.with(|cell| {
            let session = cell.borrow();
            session.animations.is_empty()
        });
        if idle {
            unsafe {
                let _ = KillTimer(Some(host), ANIMATION_TIMER_ID);
            }
        }
    }
}

fn cancel_all_animations() {
    let (host, windows) = SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        let mut windows = session
            .animations
            .iter()
            .map(|animation| animation.window)
            .collect::<Vec<_>>();
        session.animations.clear();
        if session.animation_tick_active {
            windows.extend(
                session
                    .animation_versions
                    .iter()
                    .map(|(window, _)| HWND(*window as *mut _)),
            );
        }
        (session.host, windows)
    });
    SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        for window in windows {
            bump_animation_generation(&mut session, window);
        }
    });
    if let Some(host) = host {
        unsafe {
            let _ = KillTimer(Some(host), ANIMATION_TIMER_ID);
        }
    }
}

fn start_window_animation(
    hwnd: HWND,
    from: Rect,
    to: Rect,
    settings: &Settings,
    completion: AnimationCompletion,
) -> Result<bool, String> {
    if from == to || !animations_allowed(settings) {
        return Ok(false);
    }
    if unsafe { IsHungAppWindow(hwnd).as_bool() } {
        return Err("target window is not responding".into());
    }
    let Some(host) = SESSION.with(|cell| cell.borrow().host) else {
        return Ok(false);
    };
    let history_token = history::identity_token(hwnd);
    if history_token == 0 {
        return Ok(false);
    }
    let recovery_token = unsafe { GetPropW(hwnd, RECOVERY_PROPERTY) }.0 as usize;
    let mut process_id = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }
    if process_id == 0 {
        return Ok(false);
    }
    let generation = SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        let generation = bump_animation_generation(&mut session, hwnd);
        session
            .animations
            .retain(|animation| animation.window != hwnd);
        session.animations.push(WindowAnimation {
            window: hwnd,
            process_id,
            history_token,
            recovery_token,
            generation,
            from,
            to,
            started: Instant::now(),
            duration: std::time::Duration::from_millis(u64::from(settings.animation_duration_ms)),
            completion,
        });
        generation
    });
    unsafe {
        let _ = SetTimer(Some(host), ANIMATION_TIMER_ID, 16, None);
    }
    let _ = generation;
    Ok(true)
}

fn interpolate_rect(from: Rect, to: Rect, progress: f64) -> Rect {
    let progress = progress.clamp(0.0, 1.0);
    let interpolate = |start: i32, end: i32| {
        (f64::from(start) + f64::from(end.saturating_sub(start)) * progress)
            .round()
            .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
    };
    Rect {
        left: interpolate(from.left, to.left),
        top: interpolate(from.top, to.top),
        right: interpolate(from.right, to.right),
        bottom: interpolate(from.bottom, to.bottom),
    }
}

fn advance_window_animations(host: HWND) {
    let now = Instant::now();
    let animations = SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        session.animation_tick_active = true;
        std::mem::take(&mut session.animations)
    });
    let mut unfinished = Vec::new();
    for animation in animations {
        if !animation_is_current(animation.window, animation.generation)
            || !unsafe { IsWindow(Some(animation.window)).as_bool() }
        {
            continue;
        }
        if unsafe { IsHungAppWindow(animation.window).as_bool() } {
            notify_error(host, "A window stopped responding during its animation.");
            continue;
        }
        let mut process_id = 0;
        unsafe {
            GetWindowThreadProcessId(animation.window, Some(&mut process_id));
        }
        if process_id != animation.process_id
            || history::identity_token(animation.window) != animation.history_token
            || unsafe { GetPropW(animation.window, RECOVERY_PROPERTY).0 as usize }
                != animation.recovery_token
        {
            continue;
        }
        let progress = (now.duration_since(animation.started).as_secs_f64()
            / animation.duration.as_secs_f64())
        .clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - progress).powi(3);
        let frame = interpolate_rect(animation.from, animation.to, eased);
        if let Err(error) = position_visible_frame(animation.window, frame, Default::default()) {
            notify_error(host, &error);
            continue;
        }
        if !animation_is_current(animation.window, animation.generation) {
            continue;
        }
        if progress >= 1.0 {
            if let Err(error) =
                set_visible_frame(animation.window, animation.to, Default::default())
            {
                notify_error(host, &error);
            }
            if !animation_is_current(animation.window, animation.generation) {
                continue;
            }
            if let AnimationCompletion::Unstash { entry, focus } = animation.completion {
                let result = restore_placement(entry.window, &entry.placement)
                    .and_then(|()| clear_recovery(entry.window, false));
                if let Err(error) = result {
                    notify_error(
                        host,
                        &format!("cannot finish restoring stashed window: {error}"),
                    );
                } else if focus {
                    let _ = unsafe { SetForegroundWindow(entry.window) };
                }
            }
        } else {
            unfinished.push(animation);
        }
    }
    let active = SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        for animation in unfinished {
            let current = session
                .animation_versions
                .iter()
                .any(|(window, generation)| {
                    *window == animation.window.0 as isize && *generation == animation.generation
                });
            if current
                && !session
                    .animations
                    .iter()
                    .any(|queued| queued.window == animation.window)
            {
                session.animations.push(animation);
            }
        }
        session.animation_tick_active = false;
        !session.animations.is_empty()
    });
    if !active {
        unsafe {
            let _ = KillTimer(Some(host), ANIMATION_TIMER_ID);
        }
    }
}

fn apply_frame_action(hwnd: HWND, action: Action, settings: &Settings) -> Result<(), String> {
    if !unsafe { IsWindow(Some(hwnd)).as_bool() && IsWindowVisible(hwnd).as_bool() } {
        return Err("target window is no longer available".into());
    }
    if unsafe { IsHungAppWindow(hwnd).as_bool() } {
        return Err("target window is not responding".into());
    }
    ensure_fullscreen_policy(hwnd, settings)?;
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
    if !settings.animate_window_resizes
        || !start_window_animation(hwnd, old, target, settings, AnimationCompletion::None)?
    {
        set_visible_frame(hwnd, target, Default::default())?;
    }
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
    if !hwnd.0.is_null() {
        break_shortcut_cycles_for_action(hwnd, action, settings);
    }
    if hwnd.0.is_null() && action == Action::Undo {
        if let Some(target) = with_history(|history| Ok(history.last_window()))? {
            cancel_window_animation(target);
        }
    } else {
        cancel_window_animation(hwnd);
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
            return move_to_monitor(hwnd, action, settings);
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
    let target = Rect {
        left: x,
        top: y,
        right: x.saturating_add(current.width()),
        bottom: y.saturating_add(current.height()),
    };
    let result = if settings.animate_stashed_windows {
        start_window_animation(hwnd, current, target, settings, AnimationCompletion::None).and_then(
            |animated| {
                if animated {
                    Ok(())
                } else {
                    set_visible_frame(hwnd, target, Default::default())
                }
            },
        )
    } else {
        set_visible_frame(hwnd, target, Default::default())
    };
    result.map_err(|error| {
        let _ = clear_recovery(hwnd, false);
        format!("cannot stash window: {error}")
    })?;
    if settings.shift_focus_when_stashed {
        let _ = focus_window(hwnd, Action::FocusNextInStack, settings);
    }
    Ok(())
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
    if settings.animate_stashed_windows && animations_allowed(settings) {
        let from = rect_from_window(entry.window)?;
        restore_placement(entry.window, &entry.placement)
            .map_err(|error| format!("cannot restore stashed window: {error}"))?;
        position_visible_frame(entry.window, from, Default::default())?;
        if start_window_animation(
            entry.window,
            from,
            entry.original_frame,
            settings,
            AnimationCompletion::Unstash {
                entry: entry.clone(),
                focus: settings.shift_focus_when_stashed,
            },
        )? {
            return Ok(());
        }
    }
    restore_placement(entry.window, &entry.placement)
        .map_err(|error| format!("cannot restore stashed window: {error}"))?;
    clear_recovery(entry.window, false)?;
    if settings.shift_focus_when_stashed {
        let _ = unsafe { SetForegroundWindow(entry.window) };
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

fn move_to_monitor(hwnd: HWND, action: Action, settings: &Settings) -> Result<(), String> {
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
            let center = effective_work_area(current, settings).center();

            all.iter()
                .copied()
                .filter_map(|monitor| {
                    if monitor.handle == current.handle {
                        return None;
                    }
                    let other = effective_work_area(monitor, settings).center();
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
    let destination_work = effective_work_area(destination, settings);
    let width = old.width().min(destination_work.width()).max(1);
    let height = old.height().min(destination_work.height()).max(1);
    let center = old.center();
    let x = (center.0 - width / 2).clamp(destination_work.left, destination_work.right - width);
    let y = (center.1 - height / 2).clamp(destination_work.top, destination_work.bottom - height);
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
    let target_monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    let origin = target_rect.center();
    let mut candidates = Vec::new();
    for (z_order, candidate) in windows.into_iter().enumerate() {
        if candidate == hwnd
            || !unsafe { IsWindowVisible(candidate).as_bool() }
            || unsafe { IsIconic(candidate).as_bool() }
            || is_protected_window(candidate)
            || is_excluded(candidate, settings).unwrap_or(true)
            || SESSION.with(|cell| {
                let session = cell.borrow();
                session
                    .stash
                    .iter()
                    .chain(&session.hidden)
                    .any(|entry| entry.window == candidate)
            })
        {
            continue;
        }
        if action == Action::FocusNextInStack
            && unsafe { MonitorFromWindow(candidate, MONITOR_DEFAULTTONEAREST) } != target_monitor
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
    TRIGGER_KEY.store(u32::from(settings.trigger.key), Ordering::Release);
    TRIGGER_KEY_HELD.store(false, Ordering::Release);
    if settings.trigger.key != 0 {
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
    }
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
            progress: Vec::new(),
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
            let cycle_restart = session.settings.cycle_restart;
            let settings = session.settings.clone();
            let index = session
                .shortcuts
                .iter()
                .position(|cycle| cycle.ids.contains(&id))?;
            let now = Instant::now();
            let target = unsafe { GetForegroundWindow() };
            if target.0.is_null() {
                return None;
            }
            let mut process_id = 0;
            unsafe {
                GetWindowThreadProcessId(target, Some(&mut process_id));
            }
            if process_id == 0 {
                return None;
            }
            let token = history::identity_token(target);
            if cycle_restart {
                for (other_index, cycle) in session.shortcuts.iter_mut().enumerate() {
                    if other_index != index {
                        cycle.progress.retain(|progress| progress.window != target);
                    }
                }
            }
            let cycle = &mut session.shortcuts[index];
            cycle.progress.retain(|progress| {
                progress.window != target
                    || shortcut_progress_matches(
                        progress,
                        target,
                        process_id,
                        token,
                        now,
                        std::time::Duration::from_millis(u64::from(timeout_ms)),
                    )
            });
            let progress_index = cycle
                .progress
                .iter()
                .position(|progress| progress.window == target);
            let next = progress_index.map_or(0, |index| cycle.progress[index].next);
            let reverse = cycle_backwards && cycle.ids.get(1) == Some(&id);
            let index = cycle_action_index(next, cycle.actions.len(), reverse);
            let action = cycle.actions[index];
            let next = if reverse {
                index
            } else {
                (index + 1) % cycle.actions.len()
            };
            let progress = ShortcutProgress {
                window: target,
                process_id,
                token,
                next,
                last_press: now,
            };
            if let Some(progress_index) = progress_index {
                cycle.progress[progress_index] = progress;
            } else {
                cycle.progress.push(progress);
            }
            if cycle.progress.len() > 128 {
                cycle.progress.remove(0);
            }
            Some((target, action, settings))
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
    let token = history::identity_token(target);
    if token != 0 {
        let mut process_id = 0;
        unsafe {
            GetWindowThreadProcessId(target, Some(&mut process_id));
        }
        if process_id != 0 {
            SESSION.with(|cell| {
                refresh_cycle_identity(&mut cell.borrow_mut().shortcuts, target, process_id, token);
            });
        }
    }
    let _ = hwnd;
}

fn reset_other_shortcut_cycles(active_id: i32) {
    let target = unsafe { GetForegroundWindow() };
    SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        if !session.settings.cycle_restart {
            return;
        }
        for cycle in &mut session.shortcuts {
            if !cycle.ids.contains(&active_id) {
                cycle.progress.retain(|progress| progress.window != target);
            }
        }
    });
}

fn break_shortcut_cycles_for_action(target: HWND, action: Action, settings: &Settings) {
    if !settings.cycle_restart {
        return;
    }
    let belongs_to_cycle = SESSION.with(|cell| {
        cell.borrow()
            .shortcuts
            .iter()
            .any(|cycle| cycle.actions.contains(&action))
    });
    if belongs_to_cycle {
        return;
    }
    SESSION.with(|cell| {
        for cycle in &mut cell.borrow_mut().shortcuts {
            cycle.progress.retain(|progress| progress.window != target);
        }
    });
}

fn input_key_down(key: u16) -> bool {
    unsafe { GetAsyncKeyState(i32::from(key)) as u16 & 0x8000 != 0 }
}

fn modifier_held(left_vk: i32, right_vk: i32, generic_vk: i32, side: TriggerSide) -> bool {
    match side {
        TriggerSide::Either => input_key_down(generic_vk as u16),
        TriggerSide::Left => input_key_down(left_vk as u16),
        TriggerSide::Right => input_key_down(right_vk as u16),
    }
}

fn trigger_side_matches(hotkey: Hotkey, side: TriggerSide) -> bool {
    (!hotkey.control || modifier_held(0xA2, 0xA3, i32::from(VK_CONTROL.0), side))
        && (!hotkey.alt || modifier_held(0xA4, 0xA5, i32::from(VK_MENU.0), side))
        && (!hotkey.shift || modifier_held(0xA0, 0xA1, i32::from(VK_SHIFT.0), side))
        && (!hotkey.win || modifier_held(0x5B, 0x5C, 0x5B, side))
}

fn trigger_key_down(hotkey: Hotkey) -> bool {
    hotkey.key == 0 || TRIGGER_KEY_HELD.load(Ordering::Acquire) || input_key_down(hotkey.key)
}

fn trigger_held(hotkey: Hotkey, side: TriggerSide) -> bool {
    trigger_key_down(hotkey) && trigger_side_matches(hotkey, side)
}

fn double_tap_interval() -> std::time::Duration {
    std::time::Duration::from_millis(u64::from(unsafe { GetDoubleClickTime() }.min(400)))
}

fn start_trigger(host: HWND, kind: TriggerKind, settings: &Settings) {
    let delay = if kind == TriggerKind::MiddleMouse && !settings.middle_click_uses_delay {
        0
    } else if kind == TriggerKind::Keyboard
        && settings.trigger.key == 0
        && settings.trigger_delay_ms == 0
    {
        // Leave one tick so Ctrl+Alt plus a shortcut key can cancel before the ring opens.
        16
    } else {
        settings.trigger_delay_ms
    };
    SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        session.trigger_kind = kind;
        session.trigger_wait_double_tap = false;
        session.trigger_started = Some(Instant::now());
        session.trigger_pending = delay > 0;
    });
    if delay == 0 {
        begin_radial(host, kind);
    } else {
        unsafe {
            let _ = SetTimer(Some(host), TIMER_ID, 8, None);
        }
    }
}

fn start_or_complete_double_tap(host: HWND, kind: TriggerKind, settings: &Settings) {
    if !settings.double_tap_to_trigger {
        start_trigger(host, kind, settings);
        return;
    }
    let now = Instant::now();
    let completed = SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        let last = match kind {
            TriggerKind::Keyboard => &mut session.last_trigger_key_release,
            TriggerKind::MiddleMouse => &mut session.last_middle_release,
        };
        if last.is_some_and(|last| now.duration_since(last) <= double_tap_interval()) {
            *last = None;
            true
        } else {
            *last = None;
            session.trigger_kind = kind;
            session.trigger_wait_double_tap = true;
            session.trigger_pending = true;
            session.trigger_started = Some(now);
            false
        }
    });
    if completed {
        start_trigger(host, kind, settings);
    } else {
        unsafe {
            let _ = SetTimer(Some(host), TIMER_ID, 8, None);
        }
    }
}

fn begin_radial(host: HWND, kind: TriggerKind) {
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
    // Ignore fullscreen input quietly, as other desktop shortcuts do during a game.
    if settings.ignore_fullscreen
        && monitor_info(unsafe { MonitorFromWindow(target, MONITOR_DEFAULTTONEAREST) })
            .is_ok_and(|monitor| is_fullscreen(target, monitor))
    {
        return;
    }
    if let Err(error) = ensure_target(target, &settings) {
        notify_error(host, &error);
        return;
    }
    // Fast-fail hung targets before GetWindowPlacement can block the UI thread.
    if unsafe { IsHungAppWindow(target).as_bool() } {
        notify_error(host, "the target window is not responding");
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
        session.radial_bitmap = None;
        session.arrow_state = 0;
        session.open = true;
        session.trigger_pending = false;
        session.trigger_kind = kind;
        session.trigger_wait_double_tap = false;
        session.trigger_started = Some(Instant::now());
        session.last_preview_full_build = None;
        session.overlay
    });
    if let Some(overlay) = overlay {
        let dpi = unsafe { windows::Win32::UI::HiDpi::GetDpiForWindow(target) }.max(96);
        let size = crate::radial::window_size_px(&settings, dpi);
        let show_ring = settings.radial_menu_visible && !settings.hide_on_no_selection;
        let _ = unsafe {
            SetWindowPos(
                overlay,
                Some(HWND_TOPMOST),
                origin.0 - size / 2,
                origin.1 - size / 2,
                size,
                size,
                SWP_NOACTIVATE,
            )
        };
        if show_ring {
            unsafe {
                let _ = ShowWindow(overlay, SW_SHOWNOACTIVATE);
            }
            redraw_radial(host);
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
    TASKBAR_CREATED_MESSAGE.store(
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW(w!("TaskbarCreated"))
        },
        Ordering::Release,
    );
    // A layered topmost popup needs WS_EX_TRANSPARENT to pass input to other processes.
    // HTTRANSPARENT in window_proc only searches windows on this thread.
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
    let overlay = unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT,
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
    enable_color_key(preview, settings.preview_opacity)?;
    enable_color_key(overlay, RADIAL_WINDOW_OPACITY)?;
    SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        session.overlay = Some(overlay);
        session.preview = Some(preview);
        session.preview_layer_opacity = Some(settings.preview_opacity);
        session.host = Some(host);
        session.settings = settings.clone();
    });
    // Render all ring states once at startup so the first hotkey is a cache
    // hit plus one small blit instead of a synchronous compose.
    prewarm_radial_cache(&settings, 96);
    MIDDLE_CLICK_ENABLED.store(settings.middle_click_triggers, Ordering::Release);
    SESSION.with(|cell| cell.borrow_mut().tray_icon = Some(class.hIcon));
    if !settings.hide_tray_icon {
        set_tray_icon(host, true)?;
    }
    if let Some(error) = recovery_error.as_deref() {
        notify_error(host, error);
    }
    let shortcuts = match register_hotkeys(host, &settings) {
        Ok(shortcuts) => shortcuts,
        Err(error) => {
            unsafe {
                let _ = set_tray_icon(host, false);
                let _ = DestroyWindow(overlay);
                let _ = DestroyWindow(preview);
                let _ = DestroyWindow(host);
            }
            return Err(error);
        }
    };
    SESSION.with(|cell| cell.borrow_mut().shortcuts = shortcuts);
    HOOK_HOST.store(host.0 as usize, Ordering::Relaxed);
    MIDDLE_CLICK_HELD.store(false, Ordering::Release);
    let keyboard_hook = match unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), Some(instance), 0)
    } {
        Ok(hook) => hook,
        Err(error) => {
            HOOK_HOST.store(0, Ordering::Relaxed);
            unregister_hotkeys(host);
            unsafe {
                let _ = set_tray_icon(host, false);
                let _ = DestroyWindow(overlay);
                let _ = DestroyWindow(preview);
                let _ = DestroyWindow(host);
            }
            return Err(format!("cannot install Orbit keyboard hook: {error}"));
        }
    };
    let mouse_hook =
        match unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), Some(instance), 0) } {
            Ok(hook) => hook,
            Err(error) => {
                HOOK_HOST.store(0, Ordering::Relaxed);
                let _ = unsafe { UnhookWindowsHookEx(keyboard_hook) };
                unregister_hotkeys(host);
                unsafe {
                    let _ = set_tray_icon(host, false);
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
    MIDDLE_CLICK_ENABLED.store(false, Ordering::Release);
    MIDDLE_CLICK_HELD.store(false, Ordering::Release);
    let _ = unsafe { UnhookWindowsHookEx(keyboard_hook) };
    let _ = unsafe { UnhookWindowsHookEx(mouse_hook) };
    if let Err(error) = restore_recoverable_on_exit() {
        notify_error(host, &error);
    }
    let _ = set_tray_icon(host, false);
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
    let original_frame = rect_from_window(hwnd)?;
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
        original_frame,
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
                original_frame: Rect {
                    left: entry.placement.normal_position.left,
                    top: entry.placement.normal_position.top,
                    right: entry.placement.normal_position.right,
                    bottom: entry.placement.normal_position.bottom,
                },
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
        WM_NCHITTEST => {
            let pass_through = SESSION.with(|cell| {
                let session = cell.borrow();
                session.preview == Some(hwnd) || session.overlay == Some(hwnd)
            });
            if pass_through {
                return LRESULT(HTTRANSPARENT as isize);
            }
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        message if message != 0 && message == TASKBAR_CREATED_MESSAGE.load(Ordering::Acquire) => {
            restore_tray_after_explorer_restart(hwnd);
            LRESULT(0)
        }
        WM_CLOSE => {
            if SESSION.with(|cell| cell.borrow().host == Some(hwnd)) {
                MIDDLE_CLICK_HELD.store(false, Ordering::Release);
                finish_session(hwnd, false);
                cancel_all_animations();
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
        TRIGGER_EVAL_MESSAGE => {
            evaluate_modifier_trigger(hwnd, wparam.0 as u32, lparam.0 != 0);
            LRESULT(0)
        }
        WM_HOTKEY if wparam.0 == HOTKEY_ID as usize => {
            if SESSION.with(|cell| cell.borrow().open || cell.borrow().trigger_pending) {
                return LRESULT(0);
            }
            let settings = SESSION.with(|cell| cell.borrow().settings.clone());
            if trigger_side_matches(settings.trigger, settings.trigger_side) {
                start_or_complete_double_tap(hwnd, TriggerKind::Keyboard, &settings);
            }
            LRESULT(0)
        }
        WM_HOTKEY => {
            reset_other_shortcut_cycles(wparam.0 as i32);
            register_shortcut_action(hwnd, wparam.0 as i32);
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == TIMER_ID => {
            let (pending, opened, trigger, side, delay, start, kind, waiting_double, timeout) =
                SESSION.with(|cell| {
                    let session = cell.borrow();
                    (
                        session.trigger_pending,
                        session.open,
                        session.settings.trigger,
                        session.settings.trigger_side,
                        session.settings.trigger_delay_ms,
                        session.trigger_started,
                        session.trigger_kind,
                        session.trigger_wait_double_tap,
                        session.settings.trigger_timeout_ms,
                    )
                });
            if pending {
                let held = match kind {
                    TriggerKind::Keyboard => trigger_held(trigger, side),
                    TriggerKind::MiddleMouse => MIDDLE_CLICK_HELD.load(Ordering::Acquire),
                };
                if !held {
                    SESSION.with(|cell| {
                        let mut session = cell.borrow_mut();
                        session.trigger_pending = false;
                        session.trigger_started = None;
                        if waiting_double && kind == TriggerKind::Keyboard {
                            session.last_trigger_key_release = Some(Instant::now());
                        } else if waiting_double && kind == TriggerKind::MiddleMouse {
                            session.last_middle_release = Some(Instant::now());
                        }
                    });
                    unsafe {
                        let _ = KillTimer(Some(hwnd), TIMER_ID);
                    }
                } else if !waiting_double
                    && start.is_some_and(|instant| {
                        instant.elapsed().as_millis()
                            >= u128::from(armed_trigger_delay_ms(kind, &trigger, delay))
                    })
                {
                    begin_radial(hwnd, kind);
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
            let timed_out = timeout > 0
                && SESSION
                    .with(|cell| cell.borrow().trigger_started)
                    .is_some_and(|started| started.elapsed().as_millis() >= u128::from(timeout));
            if timed_out {
                finish_session(hwnd, true);
                return LRESULT(0);
            }
            if !settings.disable_cursor_interaction {
                let mut point = POINT::default();
                if unsafe { GetCursorPos(&mut point) }.is_ok() {
                    select_radial_cursor(hwnd, (point.x, point.y));
                }
            }
            poll_radial_arrows(hwnd);
            let kind = SESSION.with(|cell| cell.borrow().trigger_kind);
            let released = match kind {
                TriggerKind::Keyboard => !trigger_held(settings.trigger, settings.trigger_side),
                // The low-level hook consumes the middle button, so GetAsyncKeyState stays up.
                TriggerKind::MiddleMouse => !MIDDLE_CLICK_HELD.load(Ordering::Acquire),
            };
            if released {
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
        WM_TIMER if wparam.0 == ANIMATION_TIMER_ID => {
            advance_window_animations(hwnd);
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == PREVIEW_ANIMATION_TIMER_ID => {
            advance_preview_animation(hwnd);
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
                    // Hook-driven cursor tracking: update the ring immediately
                    // instead of waiting for the next 16 ms timer poll.
                    if event.message == WM_MOUSEMOVE
                        && SESSION.with(|cell| {
                            cell.borrow().open
                                && !cell.borrow().settings.disable_cursor_interaction
                        })
                    {
                        select_radial_cursor(hwnd, (event.point.x, event.point.y));
                    }
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
            let role = SESSION.with(|cell| {
                let session = cell.borrow();
                if session.preview == Some(hwnd) {
                    1
                } else if session.overlay == Some(hwnd) {
                    2
                } else {
                    0
                }
            });
            match role {
                1 => repaint_layered_preview(hwnd),
                2 => repaint_layered_radial(hwnd),
                _ => validate_paint(hwnd),
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

/// Keep the previous sector until the cursor moves ~4 deg past the 22.5 deg
/// boundary. Prevents flicker when hovering on an edge; rapid flicks still switch.
fn apply_sector_hysteresis(previous: Option<usize>, raw: Option<usize>, dx: f64, dy: f64) -> Option<usize> {
    let (Some(prev), Some(next)) = (previous, raw) else {
        return raw;
    };
    if prev == next {
        return raw;
    }
    let angle = dy.atan2(dx);
    let center = prev as f64 * std::f64::consts::FRAC_PI_4;
    let mut delta = (angle - center).abs() % std::f64::consts::TAU;
    if delta > std::f64::consts::PI {
        delta = std::f64::consts::TAU - delta;
    }
    // 22.5 deg boundary + 4 deg stickiness.
    if delta <= 26.5 * std::f64::consts::PI / 180.0 {
        Some(prev)
    } else {
        raw
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
    let raw_sector = if dx * dx + dy * dy < 30.0 * 30.0 {
        None
    } else {
        Some(((dy.atan2(dx) / std::f64::consts::FRAC_PI_4).round() as i32).rem_euclid(8) as usize)
    };
    let sector = SESSION.with(|cell| {
        let session = cell.borrow();
        apply_sector_hysteresis(session.selected_sector, raw_sector, dx, dy)
    });
    let next = sector.map(|index| actions[index]);
    let changed = SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        if session.selected_sector == sector && session.selected == next {
            return false;
        }
        session.selected_sector = sector;
        session.selected = next;
        session.trigger_started = Some(Instant::now());
        true
    });
    if changed {
        let (overlay, visible, hide_no_selection) = SESSION.with(|cell| {
            let session = cell.borrow();
            (
                session.overlay,
                session.settings.radial_menu_visible,
                session.settings.hide_on_no_selection,
            )
        });
        if let Some(overlay) = overlay {
            let is_visible = unsafe { IsWindowVisible(overlay).as_bool() };
            unsafe {
                if hide_no_selection && next.is_none() {
                    if is_visible {
                        let _ = ShowWindow(overlay, SW_HIDE);
                    }
                } else if visible && !is_visible {
                    let _ = ShowWindow(overlay, SW_SHOWNOACTIVATE);
                }
            }
        }
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
        // A monitor/DPI change mid-drag leaves the window at the old size and
        // forces a blurry StretchDIBits path. Re-seat it before painting.
        let expected = crate::radial::window_size_px(&settings, dpi);
        let origin = SESSION.with(|cell| cell.borrow().origin);
        let mut rect = RECT::default();
        if unsafe { GetWindowRect(overlay, &mut rect) }.is_ok()
            && (rect.right - rect.left != expected || rect.bottom - rect.top != expected)
        {
            unsafe {
                let _ = SetWindowPos(
                    overlay,
                    Some(HWND_TOPMOST),
                    origin.0 - expected / 2,
                    origin.1 - expected / 2,
                    expected,
                    expected,
                    SWP_NOACTIVATE,
                );
            }
        }
        let key = radial_cache_key_for(&settings, dpi);
        let slot = sector.unwrap_or(8);
        if let Some(cached) = SESSION.with(|cell| {
            let session = cell.borrow();
            if session.radial_cache_key == Some(key) {
                session
                    .radial_cache
                    .as_ref()
                    .and_then(|cache| cache.get(slot).cloned().flatten())
            } else {
                None
            }
        }) {
            SESSION.with(|cell| cell.borrow_mut().radial_bitmap = Some(cached));
            unsafe {
                let _ = InvalidateRect(Some(overlay), None, false);
                let _ = UpdateWindow(overlay);
            }
            return;
        }
        match crate::radial::compose_radial(selected, sector, &settings, dpi) {
            Ok(bitmap) => {
                let presented =
                    PresentedBitmap::from_premultiplied(bitmap.size, bitmap.size, &bitmap.pixels);
                SESSION.with(|cell| {
                    let mut session = cell.borrow_mut();
                    if session.radial_cache_key != Some(key) {
                        session.radial_cache_key = Some(key);
                        session.radial_cache = Some(vec![None; 9]);
                    }
                    if let Some(cache) = session.radial_cache.as_mut()
                        && let Some(entry) = cache.get_mut(slot)
                    {
                        *entry = Some(presented.clone());
                    }
                    session.radial_bitmap = Some(presented);
                });
                unsafe {
                    let _ = InvalidateRect(Some(overlay), None, false);
                    let _ = UpdateWindow(overlay);
                }
            }
            Err(error) => notify_error(host, &error),
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
        session.trigger_started = Some(Instant::now());
        Some(())
    });
    if changed.is_some() {
        redraw_radial(host);
        update_preview();
    }
}

/// Absolute directional select for keyboard users. Arrow keys work even when
/// `disable_cursor_interaction` skips mouse polling, mirroring Loop's keyboard
/// fallback. Edge-triggered by the caller via `arrow_state`.
fn select_radial_arrow(host: HWND, sector: usize) {
    let changed = SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        if !session.open || sector >= session.settings.radial_actions.len() {
            return false;
        }
        let next = Some(session.settings.radial_actions[sector]);
        if session.selected_sector == Some(sector) && session.selected == next {
            return false;
        }
        session.selected_sector = Some(sector);
        session.selected = next;
        session.trigger_started = Some(Instant::now());
        true
    });
    if changed {
        redraw_radial(host);
        update_preview();
    }
}

fn poll_radial_arrows(host: HWND) {
    let key_down = |vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY| unsafe {
        GetAsyncKeyState(i32::from(vk.0)) < 0
    };
    let mut bits = 0u8;
    if key_down(VK_RIGHT) {
        bits |= 1;
    }
    if key_down(VK_DOWN) {
        bits |= 2;
    }
    if key_down(VK_LEFT) {
        bits |= 4;
    }
    if key_down(VK_UP) {
        bits |= 8;
    }
    let pressed = SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        let pressed = bits & !session.arrow_state;
        session.arrow_state = bits;
        pressed
    });
    if pressed == 0 {
        return;
    }
    // Absolute directions: Right=east(0), Down=south(2), Left=west(4), Up=north(6).
    let sector = if pressed & 1 != 0 {
        0
    } else if pressed & 2 != 0 {
        2
    } else if pressed & 4 != 0 {
        4
    } else {
        6
    };
    select_radial_arrow(host, sector);
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
        WM_MBUTTONDOWN => {
            let settings = SESSION.with(|cell| cell.borrow().settings.clone());
            if settings.middle_click_triggers
                && !SESSION.with(|cell| cell.borrow().open || cell.borrow().trigger_pending)
            {
                start_or_complete_double_tap(host, TriggerKind::MiddleMouse, &settings);
            }
        }
        WM_MBUTTONUP => {
            let (open, pending, wait_double, kind) = SESSION.with(|cell| {
                let session = cell.borrow();
                (
                    session.open,
                    session.trigger_pending,
                    session.trigger_wait_double_tap,
                    session.trigger_kind,
                )
            });
            if pending && kind == TriggerKind::MiddleMouse {
                SESSION.with(|cell| {
                    let mut session = cell.borrow_mut();
                    session.trigger_pending = false;
                    session.trigger_started = None;
                    if wait_double {
                        session.last_middle_release = Some(Instant::now());
                    }
                });
                unsafe {
                    let _ = KillTimer(Some(host), TIMER_ID);
                }
            } else if open && kind == TriggerKind::MiddleMouse {
                finish_session(host, true);
            }
        }
        WM_LBUTTONDOWN => {
            let settings = SESSION.with(|cell| cell.borrow().settings.clone());
            if !settings.snap_on_drag && !settings.restore_window_frame_on_drag {
                return;
            }
            let under = unsafe { WindowFromPoint(point) };
            if under.0.is_null() {
                return;
            }
            let root = unsafe { GetAncestor(under, GA_ROOT) };
            let target = if root.0.is_null() { under } else { root };
            if ensure_target(target, &settings).is_err() {
                return;
            }
            cancel_window_animation(target);
            let mut frame = RECT::default();
            if unsafe { GetWindowRect(target, &mut frame) }.is_err() {
                return;
            }
            let Ok(start_visible) = rect_from_window(target) else {
                return;
            };
            SESSION.with(|cell| {
                cell.borrow_mut().drag = Some(DragState {
                    window: target,
                    start_cursor: point,
                    last_cursor: point,
                    start_frame: frame,
                    start_visible,
                    restore_frame: settings.restore_window_frame_on_drag,
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
            let Some((window, start, frame, start_visible, restore_frame, snap, threshold)) =
                SESSION.with(|cell| {
                    let session = cell.borrow();
                    let drag = session.drag.as_ref()?;
                    Some((
                        drag.window,
                        drag.start_cursor,
                        drag.start_frame,
                        drag.start_visible,
                        drag.restore_frame,
                        session.settings.snap_on_drag,
                        session.settings.snap_threshold,
                    ))
                })
            else {
                return;
            };
            let mut current = RECT::default();
            let moved = unsafe { GetWindowRect(window, &mut current) }.is_ok()
                && ((current.left - frame.left).abs() >= 4 || (current.top - frame.top).abs() >= 4)
                && ((point.x - start.x).abs() >= 4 || (point.y - start.y).abs() >= 4);
            if moved
                && restore_frame
                && let Ok(current_visible) = rect_from_window(window)
                && (current_visible.width() != start_visible.width()
                    || current_visible.height() != start_visible.height())
            {
                let restored = Rect {
                    left: current_visible.left,
                    top: current_visible.top,
                    right: current_visible.left.saturating_add(start_visible.width()),
                    bottom: current_visible.top.saturating_add(start_visible.height()),
                };
                if let Err(error) = set_visible_frame(window, restored, Default::default()) {
                    notify_error(host, &error);
                }
            }
            let candidate = if moved && snap {
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
                session.preview_bitmap_cache = None;
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
        let middle_enabled = MIDDLE_CLICK_ENABLED.load(Ordering::Acquire);
        if event == WM_MBUTTONDOWN && middle_enabled {
            MIDDLE_CLICK_HELD.store(true, Ordering::Release);
        } else if event == WM_MBUTTONUP {
            MIDDLE_CLICK_HELD.store(false, Ordering::Release);
        }
        let wants_middle = middle_enabled && matches!(event, WM_MBUTTONDOWN | WM_MBUTTONUP);
        let (wants_drag_event, wants_wheel, wants_radial_release, wants_radial_move) =
            SESSION.with(|cell| {
                let session = cell.borrow();
                let wants_drag = session.settings.snap_on_drag
                    || session.settings.restore_window_frame_on_drag;
                (
                    wants_drag && matches!(event, WM_LBUTTONDOWN | WM_LBUTTONUP)
                        || event == WM_MOUSEMOVE
                            && (wants_drag && session.drag.is_some() || !session.stash.is_empty()),
                    event == WM_MOUSEWHEEL
                        && session.open
                        && !session.settings.disable_cursor_interaction,
                    event == WM_LBUTTONUP && session.open,
                    // Polling at 16 ms quantizes fast flicks into jumps. Forward
                    // moves directly while the ring is open for ~1 ms tracking.
                    event == WM_MOUSEMOVE
                        && session.open
                        && !session.settings.disable_cursor_interaction,
                )
            });
        if wants_drag_event || wants_wheel || wants_radial_release || wants_middle || wants_radial_move
        {
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
                        // Coalesce to the latest move so a fast flick never
                        // replays a stale queue of intermediate positions.
                        if event == WM_MOUSEMOVE
                            && let Some(index) = events
                                .iter()
                                .rposition(|queued| queued.message == WM_MOUSEMOVE)
                        {
                            events.remove(index);
                        }
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
            if wants_middle {
                return LRESULT(1);
            }
        }
    }
    unsafe { CallNextHookEx(None, code, message, details) }
}

fn armed_trigger_delay_ms(kind: TriggerKind, trigger: &Hotkey, configured_delay_ms: u32) -> u32 {
    if kind == TriggerKind::Keyboard && trigger.key == 0 && configured_delay_ms == 0 {
        16
    } else {
        configured_delay_ms
    }
}

fn evaluate_modifier_trigger(hwnd: HWND, vk: u32, down: bool) {
    if TRIGGER_KEY.load(Ordering::Acquire) != 0 {
        return;
    }
    let settings = SESSION.with(|cell| cell.borrow().settings.clone());
    let (open, pending) = SESSION.with(|cell| {
        let session = cell.borrow();
        (session.open, session.trigger_pending)
    });
    let extra_key = down && !is_trigger_modifier(vk);
    if extra_key && pending && !open {
        SESSION.with(|cell| {
            let mut session = cell.borrow_mut();
            session.trigger_pending = false;
            session.trigger_started = None;
            session.trigger_wait_double_tap = false;
        });
        unsafe {
            let _ = KillTimer(Some(hwnd), TIMER_ID);
        }
        return;
    }
    if extra_key || open || pending || !trigger_held(settings.trigger, settings.trigger_side) {
        return;
    }
    start_or_complete_double_tap(hwnd, TriggerKind::Keyboard, &settings);
}

fn is_trigger_modifier(vk: u32) -> bool {
    matches!(vk, 0x10 | 0x11 | 0x12 | 0x5b | 0x5c | 0xa0..=0xa5)
}

unsafe extern "system" fn keyboard_hook_proc(
    code: i32,
    message: WPARAM,
    details: LPARAM,
) -> LRESULT {
    if code == HC_ACTION as i32 && details.0 != 0 {
        let data = unsafe { &*(details.0 as *const KBDLLHOOKSTRUCT) };
        let event = message.0 as u32;
        let down = matches!(event, WM_KEYDOWN | WM_SYSKEYDOWN);
        let up = matches!(event, WM_KEYUP | WM_SYSKEYUP);
        if down || up {
            let trigger_key = TRIGGER_KEY.load(Ordering::Acquire);
            if trigger_key != 0 && data.vkCode == trigger_key {
                TRIGGER_KEY_HELD.store(down, Ordering::Release);
            }
            if trigger_key == 0 {
                let host = HOOK_HOST.load(Ordering::Relaxed);
                if host != 0 {
                    let _ = unsafe {
                        PostMessageW(
                            Some(HWND(host as *mut _)),
                            TRIGGER_EVAL_MESSAGE,
                            WPARAM(data.vkCode as usize),
                            LPARAM(isize::from(down)),
                        )
                    };
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

fn tray_data(host: HWND, icon: HICON) -> NOTIFYICONDATAW {
    let mut data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: host,
        uID: 1,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: TRAY_MESSAGE,
        hIcon: icon,
        ..Default::default()
    };
    let tip: Vec<u16> = "Orbit window manager"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    data.szTip[..tip.len()].copy_from_slice(&tip);
    data
}

fn set_tray_icon(host: HWND, visible: bool) -> Result<(), String> {
    let (installed, icon) = SESSION.with(|cell| {
        let session = cell.borrow();
        (session.tray_installed, session.tray_icon)
    });
    if installed == visible {
        return Ok(());
    }
    let icon = icon.ok_or("Orbit tray icon is unavailable")?;
    let data = tray_data(host, icon);
    let operation = if visible { NIM_ADD } else { NIM_DELETE };
    if !unsafe { Shell_NotifyIconW(operation, &data).as_bool() } {
        return Err(if visible {
            "cannot create notification icon"
        } else {
            "cannot remove notification icon"
        }
        .into());
    }
    SESSION.with(|cell| cell.borrow_mut().tray_installed = visible);
    Ok(())
}

fn restore_tray_after_explorer_restart(host: HWND) {
    let (visible, icon) = SESSION.with(|cell| {
        let session = cell.borrow();
        (!session.settings.hide_tray_icon, session.tray_icon)
    });
    if visible && let Some(icon) = icon {
        let data = tray_data(host, icon);
        if unsafe { Shell_NotifyIconW(NIM_ADD, &data).as_bool() } {
            SESSION.with(|cell| cell.borrow_mut().tray_installed = true);
        }
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
    MIDDLE_CLICK_HELD.store(false, Ordering::Release);
    finish_session(host, false);
    cancel_all_animations();
    SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        session.drag = None;
        session.preview_bitmap_cache = None;
        session.radial_cache_key = None;
        session.radial_cache = None;
    });
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
    if let Err(error) = set_tray_icon(host, !next.hide_tray_icon) {
        unregister_hotkeys(host);
        let restored = register_hotkeys(host, &previous);
        let _ = set_tray_icon(host, !previous.hide_tray_icon);
        if let Ok(shortcuts) = restored {
            SESSION.with(|cell| cell.borrow_mut().shortcuts = shortcuts);
        }
        return Err(error);
    }
    SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        session.settings = next.clone();
        session.shortcuts = shortcuts;
        session.last_preview_full_build = None;
    });
    prewarm_radial_cache(&next, 96);
    MIDDLE_CLICK_ENABLED.store(next.middle_click_triggers, Ordering::Release);
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
        session.preview_bitmap_cache = None;
        session.arrow_state = 0;
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

fn capture_screen_rect(frame: Rect) -> Option<Vec<u32>> {
    let width = frame.width();
    let height = frame.height();
    let count = usize::try_from(width)
        .ok()?
        .checked_mul(usize::try_from(height).ok()?)?;
    if count == 0 || count > 33_554_432 {
        return None;
    }
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    unsafe {
        let screen = GetDC(None);
        if screen.0.is_null() {
            return None;
        }
        let memory = CreateCompatibleDC(Some(screen));
        if memory.0.is_null() {
            ReleaseDC(None, screen);
            return None;
        }
        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(Some(screen), &info, DIB_RGB_COLORS, &mut bits, None, 0);
        let result = if let Ok(bitmap) = bitmap {
            let old = SelectObject(memory, HGDIOBJ(bitmap.0));
            let captured = if old.0.is_null() || bits.is_null() {
                None
            } else if BitBlt(
                memory,
                0,
                0,
                width,
                height,
                Some(screen),
                frame.left,
                frame.top,
                SRCCOPY,
            )
            .is_ok()
            {
                Some(std::slice::from_raw_parts(bits.cast::<u32>(), count).to_vec())
            } else {
                None
            };
            if !old.0.is_null() {
                let _ = SelectObject(memory, old);
            }
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
            captured
        } else {
            None
        };
        let _ = DeleteDC(memory);
        ReleaseDC(None, screen);
        result
    }
}

fn hide_preview(preview: HWND) {
    let host = SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        session.preview_animation = None;
        session.preview_current_frame = None;
        session.host
    });
    if let Some(host) = host {
        unsafe {
            let _ = KillTimer(Some(host), PREVIEW_ANIMATION_TIMER_ID);
        }
    }
    unsafe {
        let _ = ShowWindow(preview, SW_HIDE);
    }
}

fn initial_preview_frame(target: Rect, window: HWND, settings: &Settings) -> Rect {
    let center = match settings.preview_start {
        PreviewStart::ActionCenter => target.center(),
        PreviewStart::RadialMenu => SESSION.with(|cell| {
            let session = cell.borrow();
            session
                .drag
                .as_ref()
                .map(|drag| (drag.last_cursor.x, drag.last_cursor.y))
                .unwrap_or(session.origin)
        }),
        PreviewStart::ScreenCenter => monitor_for(window, settings)
            .map(|monitor| monitor.work.center())
            .unwrap_or_else(|_| target.center()),
    };
    let (width, height) = match settings.preview_start {
        PreviewStart::ActionCenter => (
            (target.width() * 4 / 5).max(1),
            (target.height() * 4 / 5).max(1),
        ),
        PreviewStart::RadialMenu | PreviewStart::ScreenCenter => (32, 32),
    };
    Rect {
        left: center.0 - width / 2,
        top: center.1 - height / 2,
        right: center.0 - width / 2 + width,
        bottom: center.1 - height / 2 + height,
    }
}

fn present_preview(preview: HWND, window: HWND, target: Rect, settings: &Settings) {
    let now = Instant::now();
    let (host, current, already_aimed, retargeting_fast) = SESSION.with(|cell| {
        let session = cell.borrow();
        let current = session
            .preview_animation
            .map(|animation| animation.frame_at(now).0)
            .or(session.preview_current_frame);
        let retargeting_fast = session.preview_animation.is_some_and(|animation| {
            animation.to != target
                && now.saturating_duration_since(animation.started)
                    < std::time::Duration::from_millis(80)
        });
        (
            session.host,
            current,
            session
                .preview_animation
                .is_some_and(|animation| animation.to == target),
            retargeting_fast,
        )
    });
    if already_aimed {
        unsafe {
            let _ = ShowWindow(preview, SW_SHOWNOACTIVATE);
            let _ = InvalidateRect(Some(preview), None, false);
            let _ = UpdateWindow(preview);
        }
        return;
    }
    let start = current.unwrap_or_else(|| initial_preview_frame(target, window, settings));
    // Keyboard-initiated shows happen 100+ times/day: first show is instant.
    // Retargets while dragging keep a short ease-out for spatial consistency.
    // A second retarget within 80 ms snaps: chasing a moving cursor with a
    // fresh 180 ms ease on every flick is what made the preview lag the ring.
    let is_first_show = current.is_none();
    let animate =
        !is_first_show && !retargeting_fast && start != target && animations_allowed(settings);
    SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        session.preview_current_frame = Some(if animate { start } else { target });
        session.preview_animation = animate.then_some(PreviewAnimation {
            from: start,
            to: target,
            started: now,
            duration: std::time::Duration::from_millis(u64::from(
                settings.animation_duration_ms.min(200),
            )),
        });
    });
    let frame = if animate { start } else { target };
    unsafe {
        let _ = SetWindowPos(
            preview,
            None,
            frame.left,
            frame.top,
            frame.width(),
            frame.height(),
            SWP_NOACTIVATE | SWP_NOZORDER,
        );
        let _ = ShowWindow(preview, SW_SHOWNOACTIVATE);
        let _ = InvalidateRect(Some(preview), None, false);
        let _ = UpdateWindow(preview);
        if let Some(host) = host {
            if animate {
                let _ = SetTimer(Some(host), PREVIEW_ANIMATION_TIMER_ID, 16, None);
            } else {
                let _ = KillTimer(Some(host), PREVIEW_ANIMATION_TIMER_ID);
            }
        }
    }
}

fn advance_preview_animation(host: HWND) {
    let (preview, animation) = SESSION.with(|cell| {
        let session = cell.borrow();
        (session.preview, session.preview_animation)
    });
    let (Some(preview), Some(animation)) = (preview, animation) else {
        unsafe {
            let _ = KillTimer(Some(host), PREVIEW_ANIMATION_TIMER_ID);
        }
        return;
    };
    let (frame, finished) = animation.frame_at(Instant::now());
    unsafe {
        let _ = SetWindowPos(
            preview,
            None,
            frame.left,
            frame.top,
            frame.width().max(1),
            frame.height().max(1),
            SWP_NOACTIVATE | SWP_NOZORDER,
        );
        let _ = InvalidateRect(Some(preview), None, false);
        let _ = UpdateWindow(preview);
    }
    SESSION.with(|cell| {
        let mut session = cell.borrow_mut();
        session.preview_current_frame = Some(frame);
        if finished {
            session.preview_animation = None;
        }
    });
    if finished {
        unsafe {
            let _ = KillTimer(Some(host), PREVIEW_ANIMATION_TIMER_ID);
        }
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
    let Some(preview) = preview else {
        return;
    };
    // The plate is its own topmost window. A monitor-sized layered window keeps the
    // bitmap in memory and never puts those pixels on the desktop.
    let placed = selection
        .filter(|(_, action)| {
            settings.preview_visible && settings.preview_opacity > 0 && action_has_preview(*action)
        })
        .and_then(|(target, action)| {
            target_frame(target, action, &settings)
                .ok()
                .map(|frame| (target, action, frame.inset(settings.preview_padding)))
        });
    let Some((target, action, frame)) = placed else {
        hide_preview(preview);
        return;
    };
    let width = frame.width();
    let height = frame.height();
    if width <= 0 || height <= 0 {
        hide_preview(preview);
        return;
    }
    let dpi = unsafe { windows::Win32::UI::HiDpi::GetDpiForWindow(target) }.max(96);
    let accent = settings.use_system_accent.then(system_accent_rgb).flatten();
    let mut render_settings = settings.clone();
    if settings.preview_use_window_corner_radius {
        // Windows has no public API for reading another process window's actual radius;
        // use the documented rounded-corner preference's standard 8-DIP radius.
        render_settings.preview_corner_radius = 8;
    }
    let style = PreviewStyleKey {
        left: frame.left,
        top: frame.top,
        width,
        height,
        dpi,
        accent: accent.unwrap_or(settings.accent_color),
        gradient: settings.gradient_color,
        border: settings.preview_border_thickness,
        radius: render_settings.preview_corner_radius,
        opacity: settings.preview_opacity,
        use_gradient: settings.use_gradient,
        action,
    };
    let cached = SESSION
        .with(|cell| cell.borrow().preview_bitmap_cache.clone())
        .filter(|(cached_key, _, _)| *cached_key == style);
    let bitmap = match cached {
        Some((_, bitmap, has_backdrop)) => Ok((bitmap, has_backdrop)),
        None => {
            // Coalesce rapid flicks: the blurred capture below is the most
            // expensive step (~11 ms release, ~112 ms debug). When sectors
            // change faster than 40 ms, paint the cheap opaque HUD now and
            // let the next stable tick do the full blurred upgrade. The ring
            // highlight itself never waits for the preview.
            let now = Instant::now();
            let full_due = preview_full_build_due(
                SESSION.with(|cell| cell.borrow().last_preview_full_build),
                now,
            );
            // Fast path (perceived performance): the blurred capture below blocks
            // the message loop for multi-MP frames. Paint the opaque HUD plate
            // instantly first so the 100+/day trigger feels snappy, then upgrade
            // to the blurred backdrop in the same tick. The upgrade is a plain
            // repaint (start == target, no animation).
            if let Ok(fast) = crate::preview::render_bitmap_with_backdrop(
                &render_settings,
                action,
                (width as u32, height as u32),
                dpi,
                accent,
                None,
            ) {
                let presented = PresentedBitmap::from_premultiplied(
                    fast.width,
                    fast.height,
                    &fast.pixels,
                );
                SESSION.with(|cell| {
                    cell.borrow_mut().preview_bitmap_cache =
                        Some((style, presented, false))
                });
                let fast_opacity = settings.preview_opacity;
                let fast_changed = SESSION
                    .with(|cell| cell.borrow().preview_layer_opacity != Some(fast_opacity));
                if fast_changed
                    && enable_color_key(preview, fast_opacity).is_ok()
                {
                    SESSION.with(|cell| {
                        cell.borrow_mut().preview_layer_opacity = Some(fast_opacity)
                    });
                }
                present_preview(preview, target, frame, &settings);
            }
            if !full_due {
                // Deferred: the timer loop re-enters update_preview once the
                // cursor settles and the debounce window expires.
                let cached = SESSION.with(|cell| cell.borrow().preview_bitmap_cache.clone());
                if let Some((_, _bitmap, _has_backdrop)) = cached.filter(|(key, _, _)| *key == style)
                {
                    let desired_opacity = settings.preview_opacity;
                    if SESSION.with(|cell| cell.borrow().preview_layer_opacity)
                        != Some(desired_opacity)
                        && enable_color_key(preview, desired_opacity).is_ok()
                    {
                        SESSION.with(|cell| {
                            cell.borrow_mut().preview_layer_opacity = Some(desired_opacity)
                        });
                    }
                    present_preview(preview, target, frame, &settings);
                    if let Some(overlay) = SESSION.with(|cell| cell.borrow().overlay)
                        && unsafe { IsWindowVisible(overlay).as_bool() }
                    {
                        raise_layered_above_foreground(overlay);
                    }
                    return;
                }
                // Fast render failed: fall through to the full build attempt.
            }
            let overlay = SESSION.with(|cell| cell.borrow().overlay);
            let overlay_visible =
                overlay.is_some_and(|window| unsafe { IsWindowVisible(window).as_bool() });
            // Only hide the overlay when it actually overlaps the capture rect.
            // Otherwise keep the ring visible to avoid a flash on every new zone.
            let overlay_overlaps = overlay.is_some_and(|window| {
                let mut rect = RECT::default();
                if unsafe { GetWindowRect(window, &mut rect) }.is_err() {
                    return true;
                }
                rect.left < frame.right
                    && rect.right > frame.left
                    && rect.top < frame.bottom
                    && rect.bottom > frame.top
            });
            let hide_overlay = overlay_visible && overlay_overlaps;
            unsafe {
                let _ = ShowWindow(preview, SW_HIDE);
                if hide_overlay && let Some(window) = overlay {
                    let _ = ShowWindow(window, SW_HIDE);
                }
            }
            let captured = capture_screen_rect(frame);
            if hide_overlay && let Some(window) = overlay {
                unsafe {
                    let _ = ShowWindow(window, SW_SHOWNOACTIVATE);
                }
            }
            let backdrop = captured
                .and_then(|pixels| crate::preview::blur_backdrop(&pixels, width, height).ok());
            let has_backdrop = backdrop.is_some();
            let bitmap = crate::preview::render_bitmap_with_backdrop(
                &render_settings,
                action,
                (width as u32, height as u32),
                dpi,
                accent,
                backdrop.as_deref(),
            );
            bitmap.map(|bitmap| {
                let presented = PresentedBitmap::from_premultiplied(
                    bitmap.width,
                    bitmap.height,
                    &bitmap.pixels,
                );
                SESSION.with(|cell| {
                    let mut session = cell.borrow_mut();
                    session.preview_bitmap_cache =
                        Some((style, presented.clone(), has_backdrop));
                    session.last_preview_full_build = Some(now);
                });
                (presented, has_backdrop)
            })
        }
    };
    let (_bitmap, has_backdrop) = match bitmap {
        Ok(result) => result,
        Err(error) => {
            hide_preview(preview);
            if let Some(host) = SESSION.with(|cell| cell.borrow().host) {
                notify_error(host, &error);
            }
            return;
        }
    };
    let desired_opacity = if has_backdrop {
        255
    } else {
        settings.preview_opacity
    };
    let alpha_changed =
        SESSION.with(|cell| cell.borrow().preview_layer_opacity != Some(desired_opacity));
    if alpha_changed {
        if let Err(error) = enable_color_key(preview, desired_opacity) {
            if let Some(host) = SESSION.with(|cell| cell.borrow().host) {
                notify_error(host, &error);
            }
            return;
        }
        SESSION.with(|cell| cell.borrow_mut().preview_layer_opacity = Some(desired_opacity));
    }
    present_preview(preview, target, frame, &settings);
    if let Some(overlay) = SESSION.with(|cell| cell.borrow().overlay)
        && unsafe { IsWindowVisible(overlay).as_bool() }
    {
        raise_layered_above_foreground(overlay);
    }
}

fn raise_layered_above_foreground(window: HWND) {
    // The overlay is already visible on every retarget: re-showing it with
    // SWP_SHOWWINDOW churns z-order and flashes. Re-assert topmost silently.
    let visible = unsafe { IsWindowVisible(window).as_bool() };
    unsafe {
        let _ = SetWindowPos(
            window,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            if visible {
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE
            } else {
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW
            },
        );
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

fn validate_paint(hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    unsafe {
        BeginPaint(hwnd, &mut ps);
        let _ = EndPaint(hwnd, &ps);
    }
}

fn repaint_layered_preview(hwnd: HWND) {
    let bitmap = SESSION.with(|cell| {
        cell.borrow()
            .preview_bitmap_cache
            .as_ref()
            .map(|(_, bitmap, _)| bitmap.clone())
    });
    paint_color_key_bitmap(hwnd, bitmap.as_ref());
}

fn repaint_layered_radial(hwnd: HWND) {
    SESSION.with(|cell| {
        let session = cell.borrow();
        let bitmap = session.radial_bitmap.as_ref();
        paint_color_key_bitmap(hwnd, bitmap);
    });
}

fn paint_color_key_bitmap(hwnd: HWND, bitmap: Option<&PresentedBitmap>) {
    let mut paint = PAINTSTRUCT::default();
    unsafe {
        let dc = BeginPaint(hwnd, &mut paint);
        if let Some(bitmap) = bitmap {
            let width = bitmap.width;
            let height = bitmap.height;
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut client = RECT::default();
            if GetClientRect(hwnd, &mut client).is_ok() {
                let dest_width = client.right - client.left;
                let dest_height = client.bottom - client.top;
                if dest_width == width && dest_height == height {
                    SetDIBitsToDevice(
                        dc,
                        0,
                        0,
                        width as u32,
                        height as u32,
                        0,
                        0,
                        0,
                        height as u32,
                        bitmap.pixels.as_ptr().cast(),
                        &info,
                        DIB_RGB_COLORS,
                    );
                } else if dest_width > 0 && dest_height > 0 {
                    // Keep the transparent key exact while resizing the cached plate.
                    let _ = SetStretchBltMode(dc, COLORONCOLOR);
                    StretchDIBits(
                        dc,
                        0,
                        0,
                        dest_width,
                        dest_height,
                        0,
                        0,
                        width,
                        height,
                        Some(bitmap.pixels.as_ptr().cast()),
                        &info,
                        DIB_RGB_COLORS,
                        SRCCOPY,
                    );
                }
            }
        } else {
            let brush = CreateSolidBrush(TRANSPARENT_COLOR_KEY);
            let _ = FillRect(dc, &paint.rcPaint, brush);
            let _ = DeleteObject(HGDIOBJ(brush.0));
        }
        let _ = EndPaint(hwnd, &paint);
    }
}

fn color_key_pixel(premultiplied: u32) -> u32 {
    let alpha = (premultiplied >> 24) & 0xff;
    if alpha < 128 {
        return TRANSPARENT_DIB_PIXEL;
    }
    let expand = |shift| (((premultiplied >> shift) & 0xff_u32) * 255_u32 + alpha / 2) / alpha;
    let rgb = (expand(16) << 16) | (expand(8) << 8) | expand(0);
    if rgb == TRANSPARENT_DIB_PIXEL {
        rgb + 1
    } else {
        rgb
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_debounce_coalesces_rapid_retargets() {
        let now = Instant::now();
        // First build is always due.
        assert!(preview_full_build_due(None, now));
        // A retarget 10 ms later defers to the fast HUD only.
        let built = now;
        assert!(!preview_full_build_due(
            Some(built),
            built + std::time::Duration::from_millis(10)
        ));
        // After the window expires the stable sector gets its full build.
        assert!(preview_full_build_due(
            Some(built),
            built + std::time::Duration::from_millis(40)
        ));
    }

    #[test]
    fn radial_cache_key_tracks_every_render_input() {
        let settings = Settings::default();
        let key = radial_cache_key_for(&settings, 96);
        let mut thicker = settings.clone();
        thicker.radial_thickness += 1;
        assert_ne!(key, radial_cache_key_for(&thicker, 96));
        assert_ne!(key, radial_cache_key_for(&settings, 144));
    }

    #[test]
    fn sector_hysteresis_keeps_previous_near_the_boundary() {
        let point = |degrees: f64| {
            let angle = degrees * std::f64::consts::PI / 180.0;
            (angle.cos() * 100.0, angle.sin() * 100.0)
        };
        // Just past the 22.5 deg edge: raw would flip to 1, hysteresis keeps 0.
        let (dx, dy) = point(24.0);
        let raw = Some(1);
        assert_eq!(apply_sector_hysteresis(Some(0), raw, dx, dy), Some(0));
        // Well past the edge + 4 deg stickiness: switch.
        let (dx, dy) = point(30.0);
        assert_eq!(apply_sector_hysteresis(Some(0), raw, dx, dy), Some(1));
        // Dead zone and no previous selection always follow raw.
        assert_eq!(apply_sector_hysteresis(Some(0), None, 0.0, 0.0), None);
        assert_eq!(apply_sector_hysteresis(None, Some(3), -70.0, 70.0), Some(3));
    }

    #[test]
    fn preview_animation_starts_at_eighty_percent_and_retargets_from_current_frame() {
        let destination = Rect {
            left: 960,
            top: 8,
            right: 1912,
            bottom: 1024,
        };
        let starting = initial_preview_frame(
            destination,
            HWND(std::ptr::null_mut()),
            &Settings::default(),
        );
        assert_eq!(starting.width(), destination.width() * 4 / 5);
        assert_eq!(starting.height(), destination.height() * 4 / 5);
        assert_eq!(starting.center(), destination.center());

        let now = Instant::now();
        let duration = std::time::Duration::from_millis(180);
        let first = PreviewAnimation {
            from: starting,
            to: destination,
            started: now,
            duration,
        };
        assert_eq!(first.frame_at(now), (starting, false));
        let midway = first.frame_at(now + duration / 2).0;
        assert!(midway.left < starting.left && midway.left > destination.left);
        assert_eq!(first.frame_at(now + duration), (destination, true));

        let next = Rect {
            left: 8,
            right: 952,
            ..destination
        };
        let retargeted = PreviewAnimation {
            from: midway,
            to: next,
            started: now + duration / 2,
            duration,
        };
        assert_eq!(retargeted.frame_at(now + duration / 2).0, midway);
        assert_eq!(retargeted.frame_at(now + duration * 2).0, next);
    }

    #[test]
    #[ignore = "local render timing diagnostic"]
    fn profile_preview_frame() {
        let width = 944;
        let height = 1016;
        let screen: Vec<u32> = (0..width * height)
            .map(|index| 0xff20_3040 | ((index as u32) & 0x1f))
            .collect();
        let settings = Settings::default();
        for sample in 0..5 {
            let started = Instant::now();
            let backdrop = crate::preview::blur_backdrop(&screen, width, height).unwrap();
            let blurred = started.elapsed();
            let bitmap = crate::preview::render_bitmap_with_backdrop(
                &settings,
                Action::RightHalf,
                (width as u32, height as u32),
                96,
                None,
                Some(&backdrop),
            )
            .unwrap();
            let rendered = started.elapsed() - blurred;
            let _: Vec<_> = bitmap.pixels.iter().copied().map(color_key_pixel).collect();
            let converted = started.elapsed() - blurred - rendered;
            println!(
                "sample={sample} blur_ms={:.2} render_ms={:.2} convert_ms={:.2}",
                blurred.as_secs_f64() * 1000.0,
                rendered.as_secs_f64() * 1000.0,
                converted.as_secs_f64() * 1000.0
            );
        }
    }

    #[test]
    fn color_key_paint_keeps_clear_pixels_clear_and_unpremultiplies_visible_pixels() {
        assert_eq!(color_key_pixel(0), TRANSPARENT_DIB_PIXEL);
        assert_eq!(color_key_pixel(0x7f40_2010), TRANSPARENT_DIB_PIXEL);
        assert_eq!(color_key_pixel(0x8080_4000), 0x00ff_8000);
        assert_ne!(color_key_pixel(0xff01_0203), TRANSPARENT_DIB_PIXEL);
    }

    #[test]
    fn shortcut_cycle_keeps_progress_when_first_action_creates_history_identity() {
        let window = HWND(std::ptr::dangling_mut());
        let now = Instant::now();
        let mut cycles = vec![ShortcutCycle {
            ids: vec![SHORTCUT_HOTKEY_BASE],
            actions: vec![Action::LeftHalf, Action::RightHalf],
            progress: vec![ShortcutProgress {
                window,
                process_id: 22,
                token: 0,
                next: 1,
                last_press: now,
            }],
        }];

        assert_eq!(refresh_cycle_identity(&mut cycles, window, 22, 9001), 1);
        let progress = &cycles[0].progress[0];
        assert!(shortcut_progress_matches(
            progress,
            window,
            22,
            9001,
            now + std::time::Duration::from_millis(1),
            std::time::Duration::from_secs(1),
        ));
        assert_eq!(
            cycle_action_index(progress.next, cycles[0].actions.len(), false),
            1
        );
    }

    #[test]
    fn shortcut_cycle_rejects_a_reused_window_identity() {
        let window = HWND(std::ptr::dangling_mut());
        let now = Instant::now();
        let progress = ShortcutProgress {
            window,
            process_id: 22,
            token: 9001,
            next: 1,
            last_press: now,
        };
        assert!(!shortcut_progress_matches(
            &progress,
            window,
            22,
            9002,
            now,
            std::time::Duration::from_secs(1),
        ));
    }

    struct WindowGuard(HWND);
    impl Drop for WindowGuard {
        fn drop(&mut self) {
            let _ = unsafe { DestroyWindow(self.0) };
        }
    }

    #[test]
    fn edge_padding_uses_measured_monitor_size_threshold() {
        let mut settings = Settings::default();
        settings.edge_padding = Some(orbit::settings::EdgePadding {
            top: 10,
            right: 20,
            bottom: 30,
            left: 40,
        });
        settings.padding_minimum_screen_inches = 24.0;
        let monitor = Monitor {
            handle: HMONITOR(std::ptr::null_mut()),
            work: Rect {
                left: -100,
                top: 20,
                right: 900,
                bottom: 820,
            },
            full: Rect::default(),
            physical_inches: Some(23.9),
        };
        assert_eq!(effective_work_area(monitor, &settings), monitor.work);
        let monitor = Monitor {
            physical_inches: Some(24.1),
            ..monitor
        };
        assert_eq!(
            effective_work_area(monitor, &settings),
            Rect {
                left: -60,
                top: 30,
                right: 880,
                bottom: 790
            }
        );
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
