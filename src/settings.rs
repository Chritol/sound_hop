//! The settings window: pick devices, rename them, set hotkeys and options.
//!
//! Built from standard Windows controls (list view, hotkey box, check boxes)
//! created in code, like building an Android layout in Kotlin instead of XML.
//! The window only exists while it is open; closing it frees everything.
//!
//! Flow: `open()` gets a snapshot of the config and devices. Every edit
//! changes the snapshot in memory. "Save" validates it, writes the file and
//! tells the main window (`WM_SETTINGS_CLOSED`), which reloads the config.

use std::cell::RefCell;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateFontIndirectW, DeleteObject, GetSysColor, GetSysColorBrush, MapWindowPoints, SetBkColor, COLOR_BTNFACE,
    COLOR_WINDOW, HBRUSH, HDC, HFONT,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, SetFocus};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetParent, GetSystemMetrics,
    IsDialogMessageW, LoadCursorW, LoadIconW, PostMessageW, RegisterClassExW, SetForegroundWindow, SetWindowPos,
    ShowWindow, SystemParametersInfoW, HWND_BOTTOM, IDCANCEL, IDC_ARROW, IDOK, MSG, NONCLIENTMETRICSW, SM_CXSCREEN,
    SM_CYSCREEN, SPI_GETNONCLIENTMETRICS, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE, SW_RESTORE, SW_SHOW,
    SW_SHOWNORMAL, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WINDOW_EX_STYLE, WM_CLOSE, WM_COMMAND, WM_CTLCOLORSTATIC,
    WM_DESTROY, WM_NOTIFY, WNDCLASSEXW, WS_CAPTION, WS_CHILD, WS_CLIPSIBLINGS, WS_EX_CONTROLPARENT, WS_GROUP,
    WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE,
};

use crate::audio::{Device, Flow};
use crate::autostart;
use crate::config::{self, Config, DeviceEntry};
use crate::hotkey::{self, Hotkey};
use crate::ui::{control, error_box, get_text, hinstance, scale, send, set_text, warning_box, wide};

const CLASS: PCWSTR = w!("SoundHopSettings");
const PAGE_CLASS: PCWSTR = w!("SoundHopSettingsPage");

// Window sizes at 100% scale.
const CLIENT_W: i32 = 600;
const CLIENT_H: i32 = 490;

// Raw Win32 constants for the common controls, kept here so the code reads
// like the Win32 documentation.
const BS_PUSHBUTTON: u32 = 0x0;
const BS_DEFPUSHBUTTON: u32 = 0x1;
const BS_AUTOCHECKBOX: u32 = 0x3;
const BS_GROUPBOX: u32 = 0x7;
const BM_GETCHECK: u32 = 0xF0;
const BM_SETCHECK: u32 = 0xF1;
const BN_CLICKED: u32 = 0;
const ES_AUTOHSCROLL: u32 = 0x80;
const EN_CHANGE: u32 = 0x300;
const SS_ENDELLIPSIS: u32 = 0x4000;
const SS_PATHELLIPSIS: u32 = 0x8000;
const WS_EX_CLIENTEDGE: u32 = 0x200;

const HKM_SETHOTKEY: u32 = 0x401;
const HKM_GETHOTKEY: u32 = 0x402;
const HKM_SETRULES: u32 = 0x403;
const HKCOMB_NONE: usize = 0x1;
const HKCOMB_S: usize = 0x2;
const HOTKEYF_CONTROL_ALT: isize = 0x2 | 0x4;

const TCM_INSERTITEMW: u32 = 0x133E;
const TCM_GETCURSEL: u32 = 0x130B;
const TCM_ADJUSTRECT: u32 = 0x1328;
const TCN_SELCHANGE: u32 = (-551i32) as u32;
const TCIF_TEXT: u32 = 0x1;

const LVS_REPORT: u32 = 0x1;
const LVS_SINGLESEL: u32 = 0x4;
const LVS_SHOWSELALWAYS: u32 = 0x8;
const LVS_NOSORTHEADER: u32 = 0x8000;
const LVS_EX_CHECKBOXES: isize = 0x4;
const LVS_EX_FULLROWSELECT: isize = 0x20;
const LVS_EX_DOUBLEBUFFER: isize = 0x10000;
const LVM_DELETEALLITEMS: u32 = 0x1009;
const LVM_ENSUREVISIBLE: u32 = 0x1013;
const LVM_SETITEMSTATE: u32 = 0x102B;
const LVM_SETEXTENDEDLISTVIEWSTYLE: u32 = 0x1036;
const LVM_INSERTITEMW: u32 = 0x104D;
const LVM_INSERTCOLUMNW: u32 = 0x1061;
const LVM_SETITEMTEXTW: u32 = 0x1074;
const LVN_ITEMCHANGED: u32 = (-101i32) as u32;
const LVIF_TEXT: u32 = 0x1;
const LVIF_STATE: u32 = 0x8;
const LVIS_FOCUSED: u32 = 0x1;
const LVIS_SELECTED: u32 = 0x2;
const LVIS_STATEIMAGEMASK: u32 = 0xF000;
const LVCF_WIDTH: u32 = 0x2;
const LVCF_TEXT: u32 = 0x4;

#[repr(C)]
struct NmHdr {
    hwnd_from: HWND,
    id_from: usize,
    code: u32,
}

#[repr(C)]
struct NmListView {
    hdr: NmHdr,
    item: i32,
    sub_item: i32,
    new_state: u32,
    old_state: u32,
    changed: u32,
    action: POINT,
    lparam: LPARAM,
}

#[repr(C)]
#[derive(Default)]
struct LvItem {
    mask: u32,
    item: i32,
    sub_item: i32,
    state: u32,
    state_mask: u32,
    text: *mut u16,
    text_max: i32,
    image: i32,
    lparam: isize,
    indent: i32,
    group_id: i32,
    columns: u32,
    pu_columns: *mut u32,
    col_fmt: *mut i32,
    group: i32,
}

#[repr(C)]
#[derive(Default)]
struct LvColumn {
    mask: u32,
    fmt: i32,
    cx: i32,
    text: *mut u16,
    text_max: i32,
    sub_item: i32,
    image: i32,
    order: i32,
    cx_min: i32,
    cx_default: i32,
    cx_ideal: i32,
}

#[repr(C)]
#[derive(Default)]
struct TcItem {
    mask: u32,
    state: u32,
    state_mask: u32,
    text: *mut u16,
    text_max: i32,
    image: i32,
    lparam: isize,
}

/// What the settings window needs from the app when it opens.
pub struct Snapshot {
    pub config: Config,
    /// Devices Windows reports right now, `[outputs, inputs]`.
    pub devices: [Vec<Device>; 2],
    /// Current default device per flow, `[output, input]`.
    pub defaults: [Option<String>; 2],
}

/// One line in a device list.
struct Row {
    windows_name: String,
    entry: DeviceEntry,
    managed: bool,
    available: bool,
    current: bool,
}

/// A hotkey box plus the text it was loaded from. If the user does not touch
/// the box we keep the original text, so a hand-written `Win+...` hotkey
/// (which the box cannot show) survives a save.
#[derive(Default)]
struct HotkeyBox {
    hwnd: HWND,
    loaded_value: u16,
    loaded_text: String,
}

impl HotkeyBox {
    fn load(&mut self, text: &str) {
        let value = Hotkey::parse(text).map(Hotkey::to_control).unwrap_or(0);
        self.loaded_value = value;
        self.loaded_text = text.trim().to_string();
        send(self.hwnd, HKM_SETHOTKEY, value as usize, 0);
    }

    fn text(&self) -> String {
        let value = send(self.hwnd, HKM_GETHOTKEY, 0, 0) as u16;
        if value == self.loaded_value {
            return self.loaded_text.clone();
        }
        Hotkey::from_control(value).map(Hotkey::to_text).unwrap_or_default()
    }

    fn clear(&mut self) {
        send(self.hwnd, HKM_SETHOTKEY, 0, 0);
    }
}

struct DevicePage {
    flow: Flow,
    panel: HWND,
    list: HWND,
    up: HWND,
    down: HWND,
    name: HWND,
    hotkey: HotkeyBox,
    clear: HWND,
    set_default: HWND,
    set_comms: HWND,
    cycle: HotkeyBox,
    cycle_clear: HWND,
    details: Vec<HWND>,
    rows: Vec<Row>,
    selected: Option<usize>,
}

struct GeneralPage {
    panel: HWND,
    autostart: HWND,
    notify: HWND,
    open_folder: HWND,
}

struct Settings {
    hwnd: HWND,
    main: HWND,
    closed_message: u32,
    font: HFONT,
    tab: HWND,
    pages: Vec<DevicePage>,
    general: GeneralPage,
    config: Config,
    saved: bool,
}

thread_local! {
    static SETTINGS: RefCell<Option<Settings>> = const { RefCell::new(None) };
}

/// Runs `f` with the settings state unless it is already in use further up
/// the call stack (Windows sends messages while we create or fill controls;
/// those are ignored on purpose).
fn with_settings<R>(f: impl FnOnce(&mut Settings) -> R) -> Option<R> {
    SETTINGS.with(|cell| cell.try_borrow_mut().ok().and_then(|mut s| s.as_mut().map(f)))
}

/// Brings the window to the front if it is open. Returns `false` if not.
pub fn focus() -> bool {
    let hwnd = SETTINGS.with(|cell| cell.try_borrow().ok().and_then(|s| s.as_ref().map(|s| s.hwnd)));
    match hwnd {
        Some(hwnd) => {
            unsafe {
                let _ = ShowWindow(hwnd, SW_RESTORE);
                let _ = SetForegroundWindow(hwnd);
            }
            true
        }
        None => false,
    }
}

/// Lets Tab, Enter and Esc work inside the settings window. Call from the
/// message loop; returns `true` if the message was handled.
pub fn dialog_message(msg: &MSG) -> bool {
    let hwnd = SETTINGS.with(|cell| cell.try_borrow().ok().and_then(|s| s.as_ref().map(|s| s.hwnd)));
    match hwnd {
        Some(hwnd) => unsafe { IsDialogMessageW(hwnd, msg).as_bool() },
        None => false,
    }
}

/// Opens the settings window. `closed_message` is posted to `main` when it
/// closes, with `wParam` 1 if settings were saved.
pub fn open(main: HWND, closed_message: u32, snapshot: Snapshot) {
    if focus() {
        return;
    }
    register_classes();
    unsafe {
        let font = message_font();
        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
        let mut rect = RECT { left: 0, top: 0, right: scale(CLIENT_W), bottom: scale(CLIENT_H) };
        let _ = AdjustWindowRectEx(&mut rect, style, false, WINDOW_EX_STYLE(0));
        let (w, h) = (rect.right - rect.left, rect.bottom - rect.top);
        let x = (GetSystemMetrics(SM_CXSCREEN) - w) / 2;
        let y = (GetSystemMetrics(SM_CYSCREEN) - h) / 2;
        let Ok(hwnd) = CreateWindowExW(
            WS_EX_CONTROLPARENT,
            CLASS,
            w!("SoundHop Settings"),
            style,
            x,
            y,
            w,
            h,
            None,
            None,
            Some(hinstance()),
            None,
        ) else {
            return;
        };

        let tab = control(
            hwnd,
            w!("SysTabControl32"),
            "",
            (WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_TABSTOP).0,
            0,
            (10, 10, CLIENT_W - 20, CLIENT_H - 60),
            0,
            font,
        );
        for (i, title) in ["Output devices", "Input devices", "General"].iter().enumerate() {
            let mut text = wide(title);
            let item = TcItem { mask: TCIF_TEXT, text: text.as_mut_ptr(), ..Default::default() };
            send(tab, TCM_INSERTITEMW, i, &item as *const _ as isize);
        }
        // The area inside the tab control where pages go, in window coordinates.
        let mut page_rect = RECT::default();
        let _ = GetClientRect(tab, &mut page_rect);
        send(tab, TCM_ADJUSTRECT, 0, &mut page_rect as *mut _ as isize);
        let mut corners =
            [POINT { x: page_rect.left, y: page_rect.top }, POINT { x: page_rect.right, y: page_rect.bottom }];
        MapWindowPoints(Some(tab), Some(hwnd), &mut corners);
        let page_rect = RECT { left: corners[0].x, top: corners[0].y, right: corners[1].x, bottom: corners[1].y };

        control(
            hwnd,
            w!("BUTTON"),
            "Save",
            (WS_CHILD | WS_VISIBLE | WS_TABSTOP).0 | BS_DEFPUSHBUTTON,
            0,
            (CLIENT_W - 10 - 88 - 8 - 88, CLIENT_H - 40, 88, 28),
            IDOK.0 as usize,
            font,
        );
        control(
            hwnd,
            w!("BUTTON"),
            "Cancel",
            (WS_CHILD | WS_VISIBLE | WS_TABSTOP).0 | BS_PUSHBUTTON,
            0,
            (CLIENT_W - 10 - 88, CLIENT_H - 40, 88, 28),
            IDCANCEL.0 as usize,
            font,
        );

        let pages = vec![
            DevicePage::create(hwnd, &page_rect, font, Flow::Output, &snapshot),
            DevicePage::create(hwnd, &page_rect, font, Flow::Input, &snapshot),
        ];
        let general = GeneralPage::create(hwnd, &page_rect, font, &snapshot.config);
        // Keep the tab control behind the pages so it does not paint over them.
        let _ = SetWindowPos(tab, Some(HWND_BOTTOM), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);

        let mut settings =
            Settings { hwnd, main, closed_message, font, tab, pages, general, config: snapshot.config, saved: false };
        settings.show_page(0);
        for page in &mut settings.pages {
            page.fill_list();
            page.load_details();
        }
        SETTINGS.with(|cell| *cell.borrow_mut() = Some(settings));

        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
        with_settings(|s| {
            let _ = SetFocus(Some(s.pages[0].list));
        });
    }
}

/// The font Windows uses for dialogs (Segoe UI 9pt on Windows 11), at the
/// current scale.
fn message_font() -> HFONT {
    unsafe {
        let mut ncm =
            NONCLIENTMETRICSW { cbSize: std::mem::size_of::<NONCLIENTMETRICSW>() as u32, ..Default::default() };
        let _ = SystemParametersInfoW(
            SPI_GETNONCLIENTMETRICS,
            ncm.cbSize,
            Some(&mut ncm as *mut _ as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
        CreateFontIndirectW(&ncm.lfMessageFont)
    }
}

fn register_classes() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        let icon = LoadIconW(Some(hinstance()), PCWSTR(1 as _)).unwrap_or_default();
        let cursor = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
        RegisterClassExW(&WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance(),
            lpszClassName: CLASS,
            hIcon: icon,
            hCursor: cursor,
            hbrBackground: HBRUSH((COLOR_BTNFACE.0 + 1) as usize as *mut _),
            ..Default::default()
        });
        RegisterClassExW(&WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(page_proc),
            hInstance: hinstance(),
            lpszClassName: PAGE_CLASS,
            hCursor: cursor,
            hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as usize as *mut _),
            ..Default::default()
        });
    });
}

/// A page is a plain child window that sits on the tab control. It passes
/// control notifications up to the settings window.
fn create_panel(parent: HWND, rect: &RECT, visible: bool) -> HWND {
    let mut style = WS_CHILD | WS_CLIPSIBLINGS;
    if visible {
        style |= WS_VISIBLE;
    }
    unsafe {
        CreateWindowExW(
            WS_EX_CONTROLPARENT,
            PAGE_CLASS,
            PCWSTR::null(),
            style,
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            Some(parent),
            None,
            Some(hinstance()),
            None,
        )
        .unwrap_or_default()
    }
}

extern "system" fn page_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_COMMAND | WM_NOTIFY => match GetParent(hwnd) {
                Ok(parent) => LRESULT(send(parent, msg, wparam.0, lparam.0)),
                Err(_) => LRESULT(0),
            },
            // Labels and check boxes: paint their background like the page.
            WM_CTLCOLORSTATIC => {
                SetBkColor(HDC(wparam.0 as *mut _), COLORREF(GetSysColor(COLOR_WINDOW)));
                LRESULT(GetSysColorBrush(COLOR_WINDOW).0 as isize)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as i32;
            let code = ((wparam.0 >> 16) & 0xFFFF) as u32;
            let ctl = HWND(lparam.0 as *mut _);
            if id == IDOK.0 {
                if with_settings(|s| s.save()).unwrap_or(false) {
                    unsafe {
                        let _ = DestroyWindow(hwnd);
                    }
                }
            } else if id == IDCANCEL.0 {
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
            } else {
                with_settings(|s| s.on_command(ctl, code));
            }
            LRESULT(0)
        }
        WM_NOTIFY => {
            // SAFETY: for WM_NOTIFY, lParam points to an NMHDR (or a larger
            // struct that starts with one) owned by the sender.
            let hdr = unsafe { &*(lparam.0 as *const NmHdr) };
            if hdr.code == TCN_SELCHANGE {
                with_settings(|s| {
                    let index = send(s.tab, TCM_GETCURSEL, 0, 0);
                    s.show_page(index.max(0) as usize);
                });
            } else if hdr.code == LVN_ITEMCHANGED {
                let nm = unsafe { &*(lparam.0 as *const NmListView) };
                with_settings(|s| s.on_item_changed(hdr.hwnd_from, nm));
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
            if let Some(s) = SETTINGS.with(|cell| cell.borrow_mut().take()) {
                unsafe {
                    let _ = DeleteObject(s.font.into());
                    let _ = PostMessageW(Some(s.main), s.closed_message, WPARAM(s.saved as usize), LPARAM(0));
                }
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn is_checked(hwnd: HWND) -> bool {
    send(hwnd, BM_GETCHECK, 0, 0) == 1
}

fn set_checked(hwnd: HWND, checked: bool) {
    send(hwnd, BM_SETCHECK, checked as usize, 0);
}

fn enable(hwnd: HWND, enabled: bool) {
    unsafe {
        let _ = EnableWindow(hwnd, enabled);
    }
}

fn hotkey_box(parent: HWND, rect: (i32, i32, i32, i32), font: HFONT) -> HotkeyBox {
    let hwnd = control(parent, w!("msctls_hotkey32"), "", (WS_CHILD | WS_VISIBLE | WS_TABSTOP).0, 0, rect, 0, font);
    // A hotkey without Ctrl or Alt (e.g. just "A") would block normal typing,
    // so the box turns such keys into Ctrl+Alt+key.
    send(hwnd, HKM_SETRULES, HKCOMB_NONE | HKCOMB_S, HOTKEYF_CONTROL_ALT);
    HotkeyBox { hwnd, ..Default::default() }
}

fn label(parent: HWND, text: &str, rect: (i32, i32, i32, i32), font: HFONT) -> HWND {
    control(parent, w!("STATIC"), text, (WS_CHILD | WS_VISIBLE).0 | SS_ENDELLIPSIS, 0, rect, 0, font)
}

fn button(parent: HWND, text: &str, rect: (i32, i32, i32, i32), font: HFONT) -> HWND {
    control(parent, w!("BUTTON"), text, (WS_CHILD | WS_VISIBLE | WS_TABSTOP).0 | BS_PUSHBUTTON, 0, rect, 0, font)
}

fn checkbox(parent: HWND, text: &str, rect: (i32, i32, i32, i32), font: HFONT) -> HWND {
    control(parent, w!("BUTTON"), text, (WS_CHILD | WS_VISIBLE | WS_TABSTOP).0 | BS_AUTOCHECKBOX, 0, rect, 0, font)
}

impl DevicePage {
    fn create(parent: HWND, rect: &RECT, font: HFONT, flow: Flow, snapshot: &Snapshot) -> DevicePage {
        let panel = create_panel(parent, rect, flow == Flow::Output);
        let kind = if flow == Flow::Output { "playback" } else { "recording" };
        label(
            panel,
            &format!("Tick the {kind} devices SoundHop should show. The cycle hotkey goes through them in this order."),
            (8, 8, 544, 32),
            font,
        );
        let list = control(
            panel,
            w!("SysListView32"),
            "",
            (WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP).0
                | LVS_REPORT
                | LVS_SINGLESEL
                | LVS_SHOWSELALWAYS
                | LVS_NOSORTHEADER,
            WS_EX_CLIENTEDGE,
            (8, 42, 440, 170),
            0,
            font,
        );
        send(list, LVM_SETEXTENDEDLISTVIEWSTYLE, 0, LVS_EX_CHECKBOXES | LVS_EX_FULLROWSELECT | LVS_EX_DOUBLEBUFFER);
        for (i, (title, width)) in [("Device", 236), ("Hotkey", 120), ("Status", 76)].iter().enumerate() {
            let mut text = wide(title);
            let col = LvColumn {
                mask: LVCF_TEXT | LVCF_WIDTH,
                cx: scale(*width),
                text: text.as_mut_ptr(),
                ..Default::default()
            };
            send(list, LVM_INSERTCOLUMNW, i, &col as *const _ as isize);
        }
        let up = button(panel, "Move up", (456, 42, 96, 26), font);
        let down = button(panel, "Move down", (456, 72, 96, 26), font);

        control(
            panel,
            w!("BUTTON"),
            "Selected device",
            (WS_CHILD | WS_VISIBLE).0 | BS_GROUPBOX,
            0,
            (8, 220, 544, 114),
            0,
            font,
        );
        let name_label = label(panel, "Name:", (20, 245, 70, 20), font);
        let name = control(
            panel,
            w!("EDIT"),
            "",
            (WS_CHILD | WS_VISIBLE | WS_TABSTOP).0 | ES_AUTOHSCROLL,
            WS_EX_CLIENTEDGE,
            (96, 241, 300, 23),
            0,
            font,
        );
        let hotkey_label = label(panel, "Hotkey:", (20, 275, 70, 20), font);
        let hotkey = hotkey_box(panel, (96, 271, 180, 23), font);
        let clear = button(panel, "Clear", (284, 270, 70, 25), font);
        let set_default = checkbox(panel, "Make it the default device", (20, 304, 240, 22), font);
        let set_comms = checkbox(panel, "Make it the communications device", (264, 304, 280, 22), font);

        label(panel, "Cycle hotkey:", (8, 352, 86, 20), font);
        let cycle = hotkey_box(panel, (96, 348, 180, 23), font);
        let cycle_clear = button(panel, "Clear", (284, 347, 70, 25), font);

        let entries = match flow {
            Flow::Output => &snapshot.config.outputs,
            Flow::Input => &snapshot.config.inputs,
        };
        let devices = &snapshot.devices[flow.index()];
        let current = snapshot.defaults[flow.index()].as_deref();
        // Managed devices first, in their configured order, then the rest.
        let mut rows: Vec<Row> = entries
            .iter()
            .map(|e| {
                let device = devices.iter().find(|d| d.id == e.id);
                Row {
                    windows_name: device.map(|d| d.name.clone()).unwrap_or_else(|| e.name.clone()),
                    entry: e.clone(),
                    managed: true,
                    available: device.is_some(),
                    current: current == Some(e.id.as_str()),
                }
            })
            .collect();
        for d in devices.iter().filter(|d| !entries.iter().any(|e| e.id == d.id)) {
            rows.push(Row {
                windows_name: d.name.clone(),
                entry: DeviceEntry { id: d.id.clone(), name: d.name.clone(), ..Default::default() },
                managed: false,
                available: true,
                current: current == Some(d.id.as_str()),
            });
        }

        let mut page = DevicePage {
            flow,
            panel,
            list,
            up,
            down,
            name,
            hotkey,
            clear,
            set_default,
            set_comms,
            cycle,
            cycle_clear,
            details: Vec::new(),
            rows,
            selected: None,
        };
        page.details =
            vec![name_label, page.name, hotkey_label, page.hotkey.hwnd, page.clear, page.set_default, page.set_comms];
        let cycle_text = match flow {
            Flow::Output => snapshot.config.general.cycle_output_hotkey.clone(),
            Flow::Input => snapshot.config.general.cycle_input_hotkey.clone(),
        };
        page.cycle.load(&cycle_text);
        if !page.rows.is_empty() {
            page.selected = Some(0);
        }
        page
    }

    fn status(row: &Row) -> &'static str {
        if !row.available {
            "Not available"
        } else if row.current {
            "Active"
        } else {
            ""
        }
    }

    /// Rebuilds the list from `rows`. Notifications sent while doing this are
    /// ignored because the settings state is borrowed.
    fn fill_list(&self) {
        send(self.list, LVM_DELETEALLITEMS, 0, 0);
        for (i, row) in self.rows.iter().enumerate() {
            let mut text = wide(row.entry.display_name());
            let item = LvItem { mask: LVIF_TEXT, item: i as i32, text: text.as_mut_ptr(), ..Default::default() };
            send(self.list, LVM_INSERTITEMW, 0, &item as *const _ as isize);
            self.update_row_text(i);
            // State image 2 = ticked, 1 = not ticked.
            let image = if row.managed { 2 } else { 1 };
            let selected = if self.selected == Some(i) { LVIS_SELECTED | LVIS_FOCUSED } else { 0 };
            let state = LvItem {
                mask: LVIF_STATE,
                state: (image << 12) | selected,
                state_mask: LVIS_STATEIMAGEMASK | LVIS_SELECTED | LVIS_FOCUSED,
                ..Default::default()
            };
            send(self.list, LVM_SETITEMSTATE, i, &state as *const _ as isize);
        }
        if let Some(i) = self.selected {
            send(self.list, LVM_ENSUREVISIBLE, i, 0);
        }
    }

    fn update_row_text(&self, i: usize) {
        let row = &self.rows[i];
        let hotkey = if row.managed { hotkey::normalize(&row.entry.hotkey) } else { String::new() };
        for (sub, text) in [(0, row.entry.display_name().to_string()), (1, hotkey), (2, Self::status(row).to_string())]
        {
            let mut text = wide(&text);
            let item = LvItem { sub_item: sub, text: text.as_mut_ptr(), ..Default::default() };
            send(self.list, LVM_SETITEMTEXTW, i, &item as *const _ as isize);
        }
    }

    /// Shows the selected row in the "Selected device" box.
    fn load_details(&mut self) {
        let row = self.selected.and_then(|i| self.rows.get(i));
        let managed = row.is_some_and(|r| r.managed);
        match row {
            Some(r) => {
                set_text(self.name, &r.entry.name);
                set_checked(self.set_default, r.entry.set_default);
                set_checked(self.set_comms, r.entry.set_communications);
                let text = if r.managed { r.entry.hotkey.clone() } else { String::new() };
                self.hotkey.load(&text);
            }
            None => {
                set_text(self.name, "");
                set_checked(self.set_default, false);
                set_checked(self.set_comms, false);
                self.hotkey.load("");
            }
        }
        for &hwnd in &self.details {
            enable(hwnd, managed);
        }
        let i = self.selected.unwrap_or(0);
        enable(self.up, self.selected.is_some() && i > 0);
        enable(self.down, self.selected.is_some() && i + 1 < self.rows.len());
    }

    /// Copies the "Selected device" box back into the selected row.
    fn commit_details(&mut self) {
        let Some(i) = self.selected else { return };
        let Some(row) = self.rows.get_mut(i) else { return };
        if !row.managed {
            return;
        }
        let name = get_text(self.name);
        row.entry.name = if name.trim().is_empty() { row.windows_name.clone() } else { name.trim().to_string() };
        row.entry.hotkey = self.hotkey.text();
        row.entry.set_default = is_checked(self.set_default);
        row.entry.set_communications = is_checked(self.set_comms);
        self.update_row_text(i);
    }

    fn move_selected(&mut self, delta: isize) {
        let Some(i) = self.selected else { return };
        let j = i as isize + delta;
        if j < 0 || j as usize >= self.rows.len() {
            return;
        }
        self.commit_details();
        self.rows.swap(i, j as usize);
        self.selected = Some(j as usize);
        self.fill_list();
        self.load_details();
    }

    fn hotkeys(&self) -> impl Iterator<Item = (String, String)> + '_ {
        self.rows.iter().filter(|r| r.managed).map(|r| (r.entry.hotkey.clone(), r.entry.display_name().to_string()))
    }

    fn entries(&self) -> Vec<DeviceEntry> {
        self.rows.iter().filter(|r| r.managed).map(|r| r.entry.clone()).collect()
    }
}

impl GeneralPage {
    fn create(parent: HWND, rect: &RECT, font: HFONT, config: &Config) -> GeneralPage {
        let panel = create_panel(parent, rect, false);
        let autostart = checkbox(panel, "Start SoundHop when Windows starts", (16, 16, 400, 22), font);
        let notify = checkbox(panel, "Show a short notice when the device changes", (16, 46, 400, 22), font);
        set_checked(autostart, autostart::is_enabled());
        set_checked(notify, config.general.show_notification);

        label(panel, "Settings file:", (16, 96, 400, 20), font);
        control(
            panel,
            w!("STATIC"),
            &config::path().to_string_lossy(),
            (WS_CHILD | WS_VISIBLE).0 | SS_PATHELLIPSIS,
            0,
            (16, 116, 530, 20),
            0,
            font,
        );
        let open_folder = button(panel, "Open folder", (16, 140, 110, 26), font);

        label(
            panel,
            &format!("SoundHop {}  ·  MIT License  ·  github.com/Chritol/sound_hop", env!("CARGO_PKG_VERSION")),
            (16, 340, 530, 20),
            font,
        );
        GeneralPage { panel, autostart, notify, open_folder }
    }
}

impl Settings {
    fn show_page(&mut self, index: usize) {
        for page in &mut self.pages {
            page.commit_details();
        }
        let panels = [self.pages[0].panel, self.pages[1].panel, self.general.panel];
        for (i, &panel) in panels.iter().enumerate() {
            unsafe {
                let _ = ShowWindow(panel, if i == index { SW_SHOW } else { SW_HIDE });
            }
        }
    }

    fn page_of(&self, hwnd: HWND) -> Option<usize> {
        self.pages.iter().position(|p| {
            p.list == hwnd
                || p.up == hwnd
                || p.down == hwnd
                || p.name == hwnd
                || p.hotkey.hwnd == hwnd
                || p.clear == hwnd
                || p.set_default == hwnd
                || p.set_comms == hwnd
                || p.cycle.hwnd == hwnd
                || p.cycle_clear == hwnd
        })
    }

    fn on_command(&mut self, ctl: HWND, code: u32) {
        if ctl == self.general.open_folder && code == BN_CLICKED {
            let _ = std::fs::create_dir_all(config::dir());
            let dir = wide(&config::dir().to_string_lossy());
            unsafe {
                ShellExecuteW(
                    Some(self.hwnd),
                    w!("open"),
                    PCWSTR(dir.as_ptr()),
                    PCWSTR::null(),
                    PCWSTR::null(),
                    SW_SHOWNORMAL,
                );
            }
            return;
        }
        let Some(p) = self.page_of(ctl) else { return };
        let page = &mut self.pages[p];
        match code {
            BN_CLICKED if ctl == page.up => page.move_selected(-1),
            BN_CLICKED if ctl == page.down => page.move_selected(1),
            BN_CLICKED if ctl == page.clear => {
                page.hotkey.clear();
                page.commit_details();
            }
            BN_CLICKED if ctl == page.cycle_clear => page.cycle.clear(),
            BN_CLICKED if ctl == page.set_default || ctl == page.set_comms => page.commit_details(),
            EN_CHANGE if ctl == page.name || ctl == page.hotkey.hwnd => page.commit_details(),
            _ => {}
        }
    }

    fn on_item_changed(&mut self, list: HWND, nm: &NmListView) {
        let Some(p) = self.pages.iter().position(|pg| pg.list == list) else { return };
        if nm.changed & LVIF_STATE == 0 || nm.item < 0 {
            return;
        }
        let i = nm.item as usize;

        // Tick box changed (ignore the very first state set when filling).
        let check_changed = (nm.old_state ^ nm.new_state) & LVIS_STATEIMAGEMASK != 0;
        if check_changed && nm.old_state & LVIS_STATEIMAGEMASK != 0 {
            let managed = (nm.new_state & LVIS_STATEIMAGEMASK) >> 12 == 2;
            let used = self.used_hotkeys();
            let page = &mut self.pages[p];
            page.commit_details();
            if let Some(row) = page.rows.get_mut(i) {
                row.managed = managed;
                if managed && row.entry.hotkey.trim().is_empty() {
                    row.entry.hotkey = hotkey::free_default(page.flow == Flow::Output, &used);
                }
            }
            page.update_row_text(i);
            if page.selected == Some(i) {
                page.load_details();
            }
        }

        // Selection changed.
        if nm.new_state & LVIS_SELECTED != 0 && nm.old_state & LVIS_SELECTED == 0 {
            let page = &mut self.pages[p];
            page.commit_details();
            page.selected = Some(i);
            page.load_details();
        }
    }

    fn used_hotkeys(&self) -> Vec<Hotkey> {
        let mut texts: Vec<String> = Vec::new();
        for page in &self.pages {
            texts.extend(page.hotkeys().map(|(h, _)| h));
            texts.push(page.cycle.text());
        }
        texts.iter().filter_map(|t| Hotkey::parse(t)).collect()
    }

    /// Validates and writes the settings. Returns `true` if the window can close.
    fn save(&mut self) -> bool {
        for page in &mut self.pages {
            page.commit_details();
        }

        // Every hotkey may be used only once.
        let mut seen: Vec<(Hotkey, String)> = Vec::new();
        for page in &self.pages {
            let cycle = (page.cycle.text(), format!("cycle {}s", page.flow.label().to_lowercase()));
            for (text, what) in page.hotkeys().chain(std::iter::once(cycle)) {
                let Some(hk) = Hotkey::parse(&text) else { continue };
                if let Some((_, other)) = seen.iter().find(|(h, _)| *h == hk) {
                    warning_box(
                        Some(self.hwnd),
                        &format!(
                            "{} is used for both \"{other}\" and \"{what}\".\n\nPlease change one of them.",
                            hk.to_text()
                        ),
                    );
                    return false;
                }
                seen.push((hk, what));
            }
        }

        let mut config = self.config.clone();
        config.outputs = self.pages[0].entries();
        config.inputs = self.pages[1].entries();
        config.general.cycle_output_hotkey = self.pages[0].cycle.text();
        config.general.cycle_input_hotkey = self.pages[1].cycle.text();
        config.general.show_notification = is_checked(self.general.notify);

        if let Err(e) = config::save(&config) {
            error_box(Some(self.hwnd), &format!("Could not save the settings:\n\n{e}"));
            return false;
        }
        let want_autostart = is_checked(self.general.autostart);
        if want_autostart != autostart::is_enabled() {
            if let Err(e) = autostart::set_enabled(want_autostart) {
                warning_box(
                    Some(self.hwnd),
                    &format!("Settings were saved, but autostart could not be changed:\n\n{e}"),
                );
            }
        }
        self.config = config;
        self.saved = true;
        true
    }
}
