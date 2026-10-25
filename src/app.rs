//! The core of SoundHop: a hidden window that owns the tray icon, receives
//! hotkeys and device notifications, and switches devices.
//!
//! Windows desktop apps are event driven: Windows puts messages (key pressed,
//! tray clicked, timer fired...) into a queue and our `wndproc` function
//! handles them one at a time on the UI thread. Between messages the app
//! sleeps inside `GetMessageW`, which is why it uses no CPU when idle.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::SystemTime;

use windows::core::{w, Result, PCWSTR};
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Controls::{LoadIconMetric, LIM_SMALL};
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_NOREPEAT};
use windows::Win32::UI::Shell::{
    ShellExecuteW, Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NIM_SETVERSION, NIN_SELECT, NOTIFYICONDATAW, NOTIFYICON_VERSION_4,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow, DispatchMessageW,
    GetMessageW, GetSystemMetrics, KillTimer, PostMessageW, PostQuitMessage, RegisterClassExW, RegisterWindowMessageW,
    SetForegroundWindow, SetMenuDefaultItem, SetTimer, TrackPopupMenuEx, TranslateMessage, HICON, HMENU, MF_CHECKED,
    MF_DISABLED, MF_GRAYED, MF_POPUP, MF_SEPARATOR, MF_STRING, MSG, SM_MENUDROPALIGNMENT, SW_SHOWNORMAL,
    TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTALIGN, TPM_RIGHTBUTTON, WINDOW_EX_STYLE, WM_APP,
    WM_CONTEXTMENU, WM_DESTROY, WM_HOTKEY, WM_NULL, WM_TIMER, WNDCLASSEXW, WS_OVERLAPPED,
};

use crate::audio::{Audio, Device, Flow, Role};
use crate::autostart;
use crate::config::{self, Config, DeviceEntry, LoadResult};
use crate::hotkey::Hotkey;
use crate::osd::{self, GLYPH_MIC, GLYPH_SPEAKER, GLYPH_WARNING};
use crate::settings;
use crate::ui::{copy_to_fixed, hinstance, warning_box, wide};

const CLASS: PCWSTR = w!("SoundHopMain");
const WM_TRAY: u32 = WM_APP + 1;
const WM_DEVICES_CHANGED: u32 = WM_APP + 2;
const WM_SECOND_INSTANCE: u32 = WM_APP + 3;
const WM_SETTINGS_CLOSED: u32 = WM_APP + 4;
const TIMER_DEVICES: usize = 1;
/// Device notifications come in bursts; wait this long for things to settle.
const DEVICE_SETTLE_MS: u32 = 300;
const TRAY_ID: u32 = 1;
/// Enter pressed on the tray icon (NIN_SELECT | NINF_KEY).
const NIN_KEYSELECT: u32 = 0x401;

/// Something the user can trigger from a hotkey or the tray menu.
#[derive(Debug, Clone)]
enum Action {
    /// Switch to the n-th managed device.
    Select(Flow, usize),
    /// Switch to the next managed device.
    Cycle(Flow),
    /// Make a managed device the communications device only.
    SelectCommunications(Flow, usize),
    /// Add or remove a device from the managed list.
    ToggleManaged(Flow, String, String),
    OpenSettings,
    OpenConfig,
    ReloadConfig,
    ToggleAutostart,
    ToggleNotifications,
    Exit,
}

pub struct App {
    hwnd: HWND,
    audio: Audio,
    config: Config,
    config_mtime: Option<SystemTime>,
    /// Registered hotkeys; the hotkey ID Windows sends back is index + 1.
    hotkeys: Vec<Action>,
    /// Last known default device per [flow][role], used to notice when a
    /// device we switched to disappears.
    last: [[Option<String>; 2]; 2],
    icon: HICON,
}

thread_local! {
    // The single App instance, owned by the UI thread. `RefCell` gives
    // checked mutable access at runtime (a bit like a lock that only one
    // thread ever takes).
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// Runs `f` with the app, unless it is already in use further up the call
/// stack (Windows can call `wndproc` again while a menu is open). In that case
/// the message is skipped instead of crashing.
fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|cell| cell.try_borrow_mut().ok().and_then(|mut app| app.as_mut().map(f)))
}

static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

pub fn run() -> Result<()> {
    unsafe {
        // Only one copy may run. A second start just tells the first one.
        let _mutex = CreateMutexW(None, true, w!("Local\\SoundHop.SingleInstance"))?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            if let Ok(existing) = windows::Win32::UI::WindowsAndMessaging::FindWindowW(CLASS, PCWSTR::null()) {
                let _ = PostMessageW(Some(existing), WM_SECOND_INSTANCE, WPARAM(0), LPARAM(0));
            }
            return Ok(());
        }

        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        let result = run_inner();
        CoUninitialize();
        result
    }
}

unsafe fn run_inner() -> Result<()> {
    let (config, problem) = match config::load() {
        LoadResult::Loaded(c) => (c, None),
        LoadResult::Missing => (Config::default(), None),
        LoadResult::Broken(e) => (
            Config::default(),
            Some(format!(
                "Your settings file could not be read, so SoundHop started with default settings.\n\
                 The old file was kept as config.toml.broken.\n\n{e}"
            )),
        ),
    };
    let first_run = !config::path().exists();

    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(wndproc),
        hInstance: hinstance(),
        lpszClassName: CLASS,
        ..Default::default()
    };
    RegisterClassExW(&wc);
    // A window that is never shown. It exists to receive messages.
    let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        CLASS,
        w!("SoundHop"),
        WS_OVERLAPPED,
        0,
        0,
        0,
        0,
        None,
        None,
        Some(hinstance()),
        None,
    )?;
    TASKBAR_CREATED.store(RegisterWindowMessageW(w!("TaskbarCreated")), Ordering::SeqCst);

    let mut audio = Audio::new()?;
    audio.watch(hwnd, WM_DEVICES_CHANGED);
    // Icon resource 1, embedded by build.rs, at the size the tray wants.
    let icon = LoadIconMetric(Some(hinstance()), PCWSTR(1 as _), LIM_SMALL).unwrap_or_default();

    let app =
        App { hwnd, audio, config, config_mtime: config_mtime(), hotkeys: Vec::new(), last: Default::default(), icon };
    APP.with(|cell| *cell.borrow_mut() = Some(app));
    with_app(|app| {
        app.remember_defaults();
        app.add_tray_icon();
        app.register_hotkeys(true);
    });

    if let Some(problem) = problem {
        warning_box(None, &problem);
    }
    if first_run {
        open_settings();
    }

    let mut msg = MSG::default();
    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
        // Tab, Enter and Esc in the settings window.
        if settings::dialog_message(&msg) {
            continue;
        }
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    with_app(|app| app.remove_tray_icon());
    // Dropping the app unregisters the device callback.
    APP.with(|cell| cell.borrow_mut().take());
    Ok(())
}

fn config_mtime() -> Option<SystemTime> {
    std::fs::metadata(config::path()).and_then(|m| m.modified()).ok()
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_HOTKEY => {
            let action = with_app(|app| app.hotkeys.get(wparam.0.wrapping_sub(1)).cloned()).flatten();
            if let Some(action) = action {
                perform(action);
            }
            LRESULT(0)
        }
        WM_TRAY => {
            // With NOTIFYICON_VERSION_4 the event is in the low word of lParam
            // and the click position is packed into wParam.
            let event = (lparam.0 & 0xFFFF) as u32;
            // While the settings window is open, the tray brings it forward
            // instead, so the two cannot make conflicting changes.
            if (event == NIN_SELECT || event == NIN_KEYSELECT || event == WM_CONTEXTMENU) && !settings::focus() {
                let x = (wparam.0 & 0xFFFF) as i16 as i32;
                let y = ((wparam.0 >> 16) & 0xFFFF) as i16 as i32;
                show_menu(hwnd, POINT { x, y });
            }
            LRESULT(0)
        }
        WM_DEVICES_CHANGED => {
            unsafe { SetTimer(Some(hwnd), TIMER_DEVICES, DEVICE_SETTLE_MS, None) };
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == TIMER_DEVICES => {
            unsafe {
                let _ = KillTimer(Some(hwnd), TIMER_DEVICES);
            }
            with_app(|app| app.on_devices_changed());
            LRESULT(0)
        }
        WM_SECOND_INSTANCE => {
            open_settings();
            LRESULT(0)
        }
        WM_SETTINGS_CLOSED => {
            let saved = wparam.0 != 0;
            with_app(|app| {
                if saved {
                    app.reload_config(true);
                } else {
                    app.register_hotkeys(true);
                }
            });
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        // Explorer restarted (e.g. after a crash): the tray icon must be added again.
        m if m != 0 && m == TASKBAR_CREATED.load(Ordering::SeqCst) => {
            with_app(|app| app.add_tray_icon());
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// Opens the settings window. Hotkeys are switched off while it is open, so
/// pressing a combination in a hotkey box sets it instead of switching.
fn open_settings() {
    if settings::focus() {
        return;
    }
    let Some((hwnd, snapshot)) = with_app(|app| {
        app.unregister_hotkeys();
        (app.hwnd, app.snapshot())
    }) else {
        return;
    };
    settings::open(hwnd, WM_SETTINGS_CLOSED, snapshot);
}

fn perform(action: Action) {
    if let Action::OpenSettings = action {
        open_settings();
        return;
    }
    if let Action::Exit = action {
        let hwnd = with_app(|app| app.hwnd);
        if let Some(hwnd) = hwnd {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
        return;
    }
    with_app(|app| app.perform(action));
}

// ---------------------------------------------------------------------------
// Tray menu
// ---------------------------------------------------------------------------

/// Builds the menu, shows it, and runs the chosen action. The app is not
/// borrowed while the menu is open, because the menu runs its own message
/// loop that calls `wndproc` again.
fn show_menu(hwnd: HWND, at: POINT) {
    let Some((menu, actions)) = with_app(|app| app.build_menu()) else { return };
    unsafe {
        // Required so the menu closes when the user clicks elsewhere.
        let _ = SetForegroundWindow(hwnd);
        let align = if GetSystemMetrics(SM_MENUDROPALIGNMENT) != 0 { TPM_RIGHTALIGN } else { TPM_LEFTALIGN };
        let cmd = TrackPopupMenuEx(
            menu,
            (TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | align).0,
            at.x,
            at.y,
            hwnd,
            None,
        )
        .0 as usize;
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
        if cmd > 0 {
            if let Some(action) = actions.get(cmd - 1).cloned() {
                perform(action);
            }
        }
    }
}

/// Collects menu items and the action behind each command ID.
struct MenuBuilder {
    actions: Vec<Action>,
}

impl MenuBuilder {
    fn item(&mut self, menu: HMENU, text: &str, action: Action, checked: bool, enabled: bool) {
        self.actions.push(action);
        let mut flags = MF_STRING;
        if checked {
            flags |= MF_CHECKED;
        }
        if !enabled {
            flags |= MF_GRAYED;
        }
        let text = wide(text);
        unsafe {
            let _ = AppendMenuW(menu, flags, self.actions.len(), PCWSTR(text.as_ptr()));
        }
    }

    fn header(&self, menu: HMENU, text: &str) {
        let text = wide(text);
        unsafe {
            let _ = AppendMenuW(menu, MF_STRING | MF_DISABLED, 0, PCWSTR(text.as_ptr()));
        }
    }

    fn separator(&self, menu: HMENU) {
        unsafe {
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        }
    }

    fn submenu(&self, menu: HMENU, text: &str, sub: HMENU) {
        let text = wide(text);
        unsafe {
            let _ = AppendMenuW(menu, MF_STRING | MF_POPUP, sub.0 as usize, PCWSTR(text.as_ptr()));
        }
    }
}

/// Menu text for a device, with its hotkey right-aligned after a tab.
fn menu_label(entry: &DeviceEntry, available: bool) -> String {
    // `&` has a special meaning in menus (underlined shortcut letter).
    let mut label = entry.display_name().replace('&', "&&");
    if !available {
        label.push_str("  (not available)");
    }
    if !entry.hotkey.trim().is_empty() {
        label.push('\t');
        label.push_str(&crate::hotkey::normalize(&entry.hotkey));
    }
    label
}

impl App {
    fn entries(&self, flow: Flow) -> &Vec<DeviceEntry> {
        match flow {
            Flow::Output => &self.config.outputs,
            Flow::Input => &self.config.inputs,
        }
    }

    fn entries_mut(&mut self, flow: Flow) -> &mut Vec<DeviceEntry> {
        match flow {
            Flow::Output => &mut self.config.outputs,
            Flow::Input => &mut self.config.inputs,
        }
    }

    fn build_menu(&mut self) -> (HMENU, Vec<Action>) {
        // Pick up hand edits to the config file without a restart.
        if config_mtime() != self.config_mtime {
            self.reload_config(false);
        }

        let mut b = MenuBuilder { actions: Vec::new() };
        unsafe {
            let menu = CreatePopupMenu().unwrap_or_default();
            let comms = CreatePopupMenu().unwrap_or_default();
            let manage = CreatePopupMenu().unwrap_or_default();

            let mut any_managed = false;
            for flow in Flow::ALL {
                let entries = self.entries(flow).clone();
                if entries.is_empty() {
                    continue;
                }
                any_managed = true;
                let current = self.audio.default_id(flow, Role::Default);
                let current_comms = self.audio.default_id(flow, Role::Communications);
                b.header(menu, flow.label());
                b.header(comms, flow.label());
                for (i, e) in entries.iter().enumerate() {
                    let available = self.audio.is_available(&e.id);
                    let label = menu_label(e, available);
                    b.item(menu, &label, Action::Select(flow, i), current.as_deref() == Some(&e.id), available);
                    let comms_label = label.split('\t').next().unwrap_or("").to_string();
                    b.item(
                        comms,
                        &comms_label,
                        Action::SelectCommunications(flow, i),
                        current_comms.as_deref() == Some(&e.id),
                        available,
                    );
                }
                b.separator(menu);
            }
            if !any_managed {
                b.header(menu, "No devices yet: add them under \"Manage devices\"");
                b.separator(menu);
                let _ = DestroyMenu(comms);
            } else {
                b.submenu(menu, "Communications device", comms);
            }

            // "Manage devices": every device Windows knows, ticked if managed.
            for flow in Flow::ALL {
                b.header(manage, &format!("{} devices", flow.label()));
                let managed: Vec<DeviceEntry> = self.entries(flow).clone();
                let present: Vec<Device> = self.audio.list(flow);
                for d in &present {
                    let is_managed = managed.iter().any(|e| e.id == d.id);
                    b.item(
                        manage,
                        &d.name.replace('&', "&&"),
                        Action::ToggleManaged(flow, d.id.clone(), d.name.clone()),
                        is_managed,
                        true,
                    );
                }
                // Managed devices that are unplugged right now can still be removed.
                for e in managed.iter().filter(|e| !present.iter().any(|d| d.id == e.id)) {
                    b.item(
                        manage,
                        &format!("{}  (not available)", e.display_name().replace('&', "&&")),
                        Action::ToggleManaged(flow, e.id.clone(), e.name.clone()),
                        true,
                        true,
                    );
                }
                if flow == Flow::Output {
                    b.separator(manage);
                }
            }
            b.submenu(menu, "Manage devices", manage);
            b.item(menu, "Settings...", Action::OpenSettings, false, true);
            let _ = SetMenuDefaultItem(menu, b.actions.len() as u32, 0);
            b.separator(menu);
            b.item(menu, "Edit config file", Action::OpenConfig, false, true);
            b.item(menu, "Reload config file", Action::ReloadConfig, false, true);
            b.item(
                menu,
                "Show notifications",
                Action::ToggleNotifications,
                self.config.general.show_notification,
                true,
            );
            b.item(menu, "Start with Windows", Action::ToggleAutostart, autostart::is_enabled(), true);
            b.separator(menu);
            b.item(menu, "Exit SoundHop", Action::Exit, false, true);
            (menu, b.actions)
        }
    }

    fn perform(&mut self, action: Action) {
        match action {
            Action::Select(flow, i) => self.select(flow, i),
            Action::Cycle(flow) => self.cycle(flow),
            Action::SelectCommunications(flow, i) => {
                let Some(entry) = self.entries(flow).get(i).cloned() else { return };
                if self.switch(flow, &entry, Role::Communications) {
                    self.notify(flow, "communications", entry.display_name());
                }
            }
            Action::ToggleManaged(flow, id, name) => self.toggle_managed(flow, id, name),
            Action::OpenConfig => self.open_config(),
            Action::ReloadConfig => self.reload_config(true),
            Action::ToggleNotifications => {
                self.config.general.show_notification = !self.config.general.show_notification;
                self.save_config();
            }
            Action::ToggleAutostart => {
                if let Err(e) = autostart::set_enabled(!autostart::is_enabled()) {
                    warning_box(Some(self.hwnd), &format!("Could not change the autostart setting:\n\n{e}"));
                }
            }
            Action::OpenSettings | Action::Exit => {}
        }
    }

    // -----------------------------------------------------------------------
    // Switching
    // -----------------------------------------------------------------------

    /// Switches to a managed device using its configured roles.
    fn select(&mut self, flow: Flow, i: usize) {
        let Some(entry) = self.entries(flow).get(i).cloned() else { return };
        if !self.audio.is_available(&entry.id) {
            self.notify_with(GLYPH_WARNING, "Not available", entry.display_name());
            return;
        }
        let mut switched = false;
        if entry.set_default {
            switched |= self.switch(flow, &entry, Role::Default);
        }
        if entry.set_communications {
            switched |= self.switch(flow, &entry, Role::Communications);
        }
        if switched {
            self.notify(flow, "", entry.display_name());
        }
    }

    /// Switches to the next available managed device after the current one.
    fn cycle(&mut self, flow: Flow) {
        let current = self.audio.default_id(flow, Role::Default);
        let entries = self.entries(flow);
        let start = current.and_then(|id| entries.iter().position(|e| e.id == id));
        match self.next_available(flow, start) {
            Some(i) => self.select(flow, i),
            None => self.notify_with(GLYPH_WARNING, flow.label(), "No other device available"),
        }
    }

    /// Index of the first available managed device after `after` (wrapping
    /// around), or the first available one if `after` is `None`.
    fn next_available(&self, flow: Flow, after: Option<usize>) -> Option<usize> {
        let entries = self.entries(flow);
        let n = entries.len();
        let first = after.map(|i| i + 1).unwrap_or(0);
        (0..n).map(|k| (first + k) % n).filter(|&i| Some(i) != after).find(|&i| self.audio.is_available(&entries[i].id))
    }

    fn switch(&mut self, flow: Flow, entry: &DeviceEntry, role: Role) -> bool {
        match self.audio.set_default(&entry.id, role) {
            Ok(()) => {
                self.last[flow.index()][role.index()] = Some(entry.id.clone());
                true
            }
            Err(e) => {
                self.notify_with(GLYPH_WARNING, "Could not switch", &e.message());
                false
            }
        }
    }

    fn notify(&self, flow: Flow, extra: &str, name: &str) {
        let glyph = if flow == Flow::Output { GLYPH_SPEAKER } else { GLYPH_MIC };
        let caption = if extra.is_empty() { flow.label().to_string() } else { format!("{} ({extra})", flow.label()) };
        self.notify_with(glyph, &caption, name);
    }

    fn notify_with(&self, glyph: char, caption: &str, text: &str) {
        self.update_tooltip();
        if self.config.general.show_notification {
            osd::show(glyph, caption, text, self.config.general.notification_ms);
        }
    }

    // -----------------------------------------------------------------------
    // Unplugged devices
    // -----------------------------------------------------------------------

    fn remember_defaults(&mut self) {
        for flow in Flow::ALL {
            for role in Role::ALL {
                self.last[flow.index()][role.index()] = self.audio.default_id(flow, role);
            }
        }
    }

    /// Called (after a short delay) whenever Windows reports a device change.
    /// If a managed device that was in use vanished, move to the next
    /// available managed device instead of whatever Windows picked.
    fn on_devices_changed(&mut self) {
        for flow in Flow::ALL {
            for role in Role::ALL {
                let last = self.last[flow.index()][role.index()].clone();
                let gone = last.as_ref().and_then(|id| {
                    let pos = self.entries(flow).iter().position(|e| &e.id == id)?;
                    (!self.audio.is_available(id)).then_some(pos)
                });
                if let Some(pos) = gone {
                    let lost = self.entries(flow)[pos].display_name().to_string();
                    if let Some(next) = self.next_available(flow, Some(pos)) {
                        let entry = self.entries(flow)[next].clone();
                        if self.switch(flow, &entry, role) {
                            if role == Role::Default {
                                self.notify_with(
                                    GLYPH_WARNING,
                                    &format!("{lost} not available, switched to"),
                                    entry.display_name(),
                                );
                            }
                            continue;
                        }
                    } else if role == Role::Default {
                        self.notify_with(GLYPH_WARNING, flow.label(), &format!("{lost} not available"));
                    }
                }
                self.last[flow.index()][role.index()] = self.audio.default_id(flow, role);
            }
        }
        self.update_tooltip();
    }

    // -----------------------------------------------------------------------
    // Managing devices and the config file
    // -----------------------------------------------------------------------

    fn toggle_managed(&mut self, flow: Flow, id: String, name: String) {
        let entries = self.entries_mut(flow);
        if let Some(pos) = entries.iter().position(|e| e.id == id) {
            entries.remove(pos);
        } else {
            let hotkey = self.free_default_hotkey(flow);
            self.entries_mut(flow).push(DeviceEntry { id, name, hotkey, ..Default::default() });
        }
        self.save_config();
    }

    fn free_default_hotkey(&self, flow: Flow) -> String {
        let used: Vec<Hotkey> = self.all_hotkey_texts().iter().filter_map(|t| Hotkey::parse(t)).collect();
        crate::hotkey::free_default(flow == Flow::Output, &used)
    }

    fn all_hotkey_texts(&self) -> Vec<String> {
        let g = &self.config.general;
        let mut v = vec![g.cycle_output_hotkey.clone(), g.cycle_input_hotkey.clone()];
        v.extend(self.config.outputs.iter().chain(&self.config.inputs).map(|e| e.hotkey.clone()));
        v
    }

    fn save_config(&mut self) {
        if let Err(e) = config::save(&self.config) {
            warning_box(Some(self.hwnd), &format!("Could not save the settings:\n\n{e}"));
        }
        self.config_mtime = config_mtime();
        self.register_hotkeys(true);
        self.update_tooltip();
    }

    fn reload_config(&mut self, report: bool) {
        match config::load() {
            LoadResult::Loaded(c) => self.config = c,
            LoadResult::Missing => self.config = Config::default(),
            LoadResult::Broken(e) => warning_box(
                Some(self.hwnd),
                &format!(
                    "The settings file has an error, so the previous settings stay active.\n\
                     The file was renamed to config.toml.broken.\n\n{e}"
                ),
            ),
        }
        self.config_mtime = config_mtime();
        self.register_hotkeys(report);
        self.update_tooltip();
    }

    fn open_config(&mut self) {
        if !config::path().exists() {
            self.save_config();
        }
        let path = wide(&config::path().to_string_lossy());
        unsafe {
            ShellExecuteW(
                Some(self.hwnd),
                w!("open"),
                w!("notepad.exe"),
                PCWSTR(path.as_ptr()),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
        }
    }

    // -----------------------------------------------------------------------
    // Hotkeys
    // -----------------------------------------------------------------------

    /// (Re)registers every hotkey from the config. Problems (typos, keys
    /// already taken by another program) are shown once if `report` is set.
    fn register_hotkeys(&mut self, report: bool) {
        self.unregister_hotkeys();

        let mut wanted: Vec<(String, Action, String)> = Vec::new();
        let g = &self.config.general;
        wanted.push((g.cycle_output_hotkey.clone(), Action::Cycle(Flow::Output), "cycle outputs".into()));
        wanted.push((g.cycle_input_hotkey.clone(), Action::Cycle(Flow::Input), "cycle inputs".into()));
        for flow in Flow::ALL {
            for (i, e) in self.entries(flow).iter().enumerate() {
                wanted.push((e.hotkey.clone(), Action::Select(flow, i), e.display_name().to_string()));
            }
        }

        let mut problems = Vec::new();
        for (text, action, what) in wanted {
            if text.trim().is_empty() {
                continue;
            }
            let Some(hk) = Hotkey::parse(&text) else {
                problems.push(format!("\"{text}\" ({what}) is not a valid hotkey"));
                continue;
            };
            self.hotkeys.push(action);
            let id = self.hotkeys.len() as i32;
            let ok = unsafe {
                RegisterHotKey(Some(self.hwnd), id, HOT_KEY_MODIFIERS(hk.mods) | MOD_NOREPEAT, hk.vk).is_ok()
            };
            if !ok {
                problems.push(format!("{} ({what}) is already used by another program", hk.to_text()));
            }
        }
        if report && !problems.is_empty() {
            osd::show(GLYPH_WARNING, "Hotkey problem", &problems.join("; "), 5000);
        }
    }

    fn unregister_hotkeys(&mut self) {
        unsafe {
            for id in 1..=self.hotkeys.len() {
                let _ = UnregisterHotKey(Some(self.hwnd), id as i32);
            }
        }
        self.hotkeys.clear();
    }

    fn snapshot(&self) -> settings::Snapshot {
        settings::Snapshot {
            config: self.config.clone(),
            devices: [self.audio.list(Flow::Output), self.audio.list(Flow::Input)],
            defaults: [
                self.audio.default_id(Flow::Output, Role::Default),
                self.audio.default_id(Flow::Input, Role::Default),
            ],
        }
    }

    // -----------------------------------------------------------------------
    // Tray icon
    // -----------------------------------------------------------------------

    fn tray_data(&self) -> NOTIFYICONDATAW {
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: TRAY_ID,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP,
            uCallbackMessage: WM_TRAY,
            hIcon: self.icon,
            ..Default::default()
        };
        copy_to_fixed(&mut nid.szTip, &self.tooltip());
        nid
    }

    fn tooltip(&self) -> String {
        let mut lines = vec!["SoundHop".to_string()];
        for flow in Flow::ALL {
            if let Some(id) = self.audio.default_id(flow, Role::Default) {
                let name = self
                    .entries(flow)
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| e.display_name().to_string())
                    .or_else(|| self.audio.list(flow).into_iter().find(|d| d.id == id).map(|d| d.name))
                    .unwrap_or_default();
                lines.push(format!("{}: {name}", flow.label()));
            }
        }
        lines.join("\n")
    }

    fn add_tray_icon(&self) {
        let mut nid = self.tray_data();
        unsafe {
            let _ = Shell_NotifyIconW(NIM_ADD, &nid);
            nid.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            let _ = Shell_NotifyIconW(NIM_SETVERSION, &nid);
        }
    }

    fn update_tooltip(&self) {
        let nid = self.tray_data();
        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
        }
    }

    fn remove_tray_icon(&self) {
        let nid = self.tray_data();
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
        }
    }
}
