//! The on-screen notice ("Output · FiiO K5 Pro") shown after a switch.
//!
//! It is a small borderless window created only when needed and destroyed when
//! its timer runs out, so it uses no memory while hidden. It never takes the
//! keyboard focus, so it does not interrupt games or typing.

use std::cell::RefCell;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DWM_WINDOW_CORNER_PREFERENCE,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontW, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect, GetDC, GetMonitorInfoW,
    InvalidateRect, MonitorFromPoint, ReleaseDC, SelectObject, SetBkMode, SetTextColor, CLEARTYPE_QUALITY,
    CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, DT_CALCRECT, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX,
    DT_SINGLELINE, DT_VCENTER, FF_DONTCARE, FONT_CHARSET, FW_NORMAL, FW_SEMIBOLD, HFONT, MONITORINFO,
    MONITOR_DEFAULTTOPRIMARY, OUT_DEFAULT_PRECIS, PAINTSTRUCT, TRANSPARENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetCursorPos, KillTimer, LoadCursorW, RegisterClassExW, SetTimer,
    SetWindowPos, ShowWindow, HWND_TOPMOST, IDC_ARROW, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_SHOWNOACTIVATE, WM_DESTROY,
    WM_LBUTTONDOWN, WM_PAINT, WM_TIMER, WNDCLASSEXW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::ui::{hinstance, scale, system_uses_light_theme, wide};

const CLASS: PCWSTR = w!("SoundHopNotice");
const TIMER_HIDE: usize = 1;

/// Glyphs from the "Segoe MDL2 Assets" font that ships with Windows 10/11.
pub const GLYPH_SPEAKER: char = '\u{E767}';
pub const GLYPH_MIC: char = '\u{E720}';
pub const GLYPH_WARNING: char = '\u{E7BA}';

struct Notice {
    hwnd: HWND,
    glyph: char,
    caption: String,
    text: String,
    caption_font: HFONT,
    text_font: HFONT,
    glyph_font: HFONT,
    light: bool,
}

thread_local! {
    // The one notice window, if it is currently on screen.
    static NOTICE: RefCell<Option<Notice>> = const { RefCell::new(None) };
}

fn font(height: i32, weight: i32, face: PCWSTR) -> HFONT {
    unsafe {
        CreateFontW(
            -scale(height),
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            FONT_CHARSET(DEFAULT_CHARSET.0),
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
            face,
        )
    }
}

fn text_width(font: HFONT, text: &str) -> i32 {
    let mut buf: Vec<u16> = text.encode_utf16().collect();
    let mut rect = RECT::default();
    unsafe {
        let dc = GetDC(None);
        let old = SelectObject(dc, font.into());
        DrawTextW(dc, &mut buf, &mut rect, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(dc, old);
        ReleaseDC(None, dc);
    }
    rect.right - rect.left
}

/// Shows (or updates) the notice. `caption` is the small grey line, `text`
/// the big one.
pub fn show(glyph: char, caption: &str, text: &str, duration_ms: u32) {
    register_class();
    let existing = NOTICE.with(|n| n.borrow().as_ref().map(|n| n.hwnd));
    let hwnd = match existing {
        Some(hwnd) => hwnd,
        None => {
            let hwnd = unsafe {
                CreateWindowExW(
                    WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                    CLASS,
                    w!("SoundHop"),
                    WS_POPUP,
                    0,
                    0,
                    0,
                    0,
                    None,
                    None,
                    Some(hinstance()),
                    None,
                )
            };
            let Ok(hwnd) = hwnd else { return };
            unsafe {
                // Rounded corners like other Windows 11 flyouts. Ignored on Windows 10.
                let pref = DWMWCP_ROUND;
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_WINDOW_CORNER_PREFERENCE,
                    &pref as *const _ as *const _,
                    std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
                );
            }
            NOTICE.with(|n| {
                *n.borrow_mut() = Some(Notice {
                    hwnd,
                    glyph,
                    caption: String::new(),
                    text: String::new(),
                    caption_font: font(12, FW_NORMAL.0 as i32, w!("Segoe UI")),
                    text_font: font(16, FW_SEMIBOLD.0 as i32, w!("Segoe UI")),
                    glyph_font: font(24, FW_NORMAL.0 as i32, w!("Segoe MDL2 Assets")),
                    light: system_uses_light_theme(),
                })
            });
            hwnd
        }
    };

    // Measure the text to size the window.
    let width = NOTICE.with(|n| {
        let mut n = n.borrow_mut();
        let n = n.as_mut().unwrap();
        n.glyph = glyph;
        n.caption = caption.to_string();
        n.text = text.to_string();
        let content = text_width(n.text_font, text).max(text_width(n.caption_font, caption));
        (content + scale(16 + 32 + 12 + 20)).clamp(scale(220), scale(520))
    });
    let height = scale(64);

    // Bottom centre of the screen the mouse is on, just above the taskbar.
    unsafe {
        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);
        let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTOPRIMARY);
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(monitor, &mut info);
        let work = info.rcWork;
        let x = work.left + (work.right - work.left - width) / 2;
        let y = work.bottom - height - scale(48);
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, width, height, SWP_NOACTIVATE | SWP_SHOWWINDOW);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let _ = InvalidateRect(Some(hwnd), None, true);
        SetTimer(Some(hwnd), TIMER_HIDE, duration_ms.max(300), None);
    }
}

fn register_class() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance(),
            lpszClassName: CLASS,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            ..Default::default()
        };
        RegisterClassExW(&wc);
    });
}

fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF(r as u32 | (g as u32) << 8 | (b as u32) << 16)
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let dc = BeginPaint(hwnd, &mut ps);
                NOTICE.with(|n| {
                    if let Some(n) = n.borrow().as_ref() {
                        paint(hwnd, dc, n);
                    }
                });
                let _ = EndPaint(hwnd, &ps);
                LRESULT(0)
            }
            WM_TIMER | WM_LBUTTONDOWN => {
                let _ = KillTimer(Some(hwnd), TIMER_HIDE);
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            WM_DESTROY => {
                if let Some(n) = NOTICE.with(|n| n.borrow_mut().take()) {
                    let _ = DeleteObject(n.caption_font.into());
                    let _ = DeleteObject(n.text_font.into());
                    let _ = DeleteObject(n.glyph_font.into());
                }
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

unsafe fn paint(hwnd: HWND, dc: windows::Win32::Graphics::Gdi::HDC, n: &Notice) {
    let mut client = RECT::default();
    let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut client);
    let (bg, fg, sub) = if n.light {
        (rgb(0xF3, 0xF3, 0xF3), rgb(0x1A, 0x1A, 0x1A), rgb(0x5F, 0x5F, 0x5F))
    } else {
        (rgb(0x2B, 0x2B, 0x2B), rgb(0xFF, 0xFF, 0xFF), rgb(0xC5, 0xC5, 0xC5))
    };
    let brush = CreateSolidBrush(bg);
    FillRect(dc, &client, brush);
    let _ = DeleteObject(brush.into());
    SetBkMode(dc, TRANSPARENT);

    // Icon
    SetTextColor(dc, fg);
    let old = SelectObject(dc, n.glyph_font.into());
    let mut glyph = wide(&n.glyph.to_string());
    glyph.pop();
    let mut r = RECT { left: scale(16), top: 0, right: scale(16 + 32), bottom: client.bottom };
    DrawTextW(dc, &mut glyph, &mut r, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);

    let text_left = scale(16 + 32 + 12);
    let text_right = client.right - scale(20);

    // Caption ("Output")
    SelectObject(dc, n.caption_font.into());
    SetTextColor(dc, sub);
    let mut caption: Vec<u16> = n.caption.encode_utf16().collect();
    let mut r = RECT { left: text_left, top: scale(11), right: text_right, bottom: scale(29) };
    DrawTextW(dc, &mut caption, &mut r, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);

    // Device name
    SelectObject(dc, n.text_font.into());
    SetTextColor(dc, fg);
    let mut text: Vec<u16> = n.text.encode_utf16().collect();
    let mut r = RECT { left: text_left, top: scale(28), right: text_right, bottom: scale(54) };
    DrawTextW(dc, &mut text, &mut r, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);

    SelectObject(dc, old);
}
