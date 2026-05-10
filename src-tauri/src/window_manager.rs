//! Win32 window-manager wrapper.
//!
//! All `unsafe` Win32 calls live here. Public functions return ordinary Rust
//! types (`Vec`, `Option`, `Result`) so the rest of the codebase stays clean.
//!
//! On non-Windows platforms (where this crate may be built for `cargo check`
//! during CI on Linux) every function is replaced by a no-op stub with the
//! same signature. This keeps the rest of the codebase platform-agnostic.

use serde::{Deserialize, Serialize};

/// A live top-level window discovered via `EnumWindows`. The `hwnd` field is
/// only valid for the duration of the running ProjectHub process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumeratedWindow {
    /// `HWND` cast to `isize`. Volatile across reboots.
    pub hwnd: isize,
    pub title: String,
    pub exe_path: String,
    pub class_name: String,
    pub pid: u32,
    pub minimized: bool,
}

#[cfg(windows)]
pub use windows_impl::*;

#[cfg(not(windows))]
pub use stub_impl::*;

// ------------------------------------------------------------------
// Windows implementation
// ------------------------------------------------------------------
#[cfg(windows)]
mod windows_impl {
    use super::EnumeratedWindow;

    use std::path::PathBuf;
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{
        CloseHandle, BOOL, HWND, LPARAM, MAX_PATH,
    };
    use windows::Win32::System::Threading::{
        AttachThreadInput, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetForegroundWindow, GetWindowLongW, GetWindowTextLengthW,
        GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible, SetForegroundWindow,
        ShowWindow, GWL_EXSTYLE, SW_MINIMIZE, SW_RESTORE, SW_SHOWNA, WS_EX_TOOLWINDOW,
    };

    /// Enumerate all visible top-level windows. Filters out tool windows,
    /// untitled windows, and ProjectHub itself (`projecthub.exe`).
    pub fn enumerate_windows(self_pid: Option<u32>) -> Vec<EnumeratedWindow> {
        let mut out: Vec<EnumeratedWindow> = Vec::new();
        let mut ctx = EnumCtx { out: &mut out, self_pid };
        unsafe {
            let _ = EnumWindows(Some(enum_proc), LPARAM(&mut ctx as *mut _ as isize));
        }
        out
    }

    struct EnumCtx<'a> {
        out: &'a mut Vec<EnumeratedWindow>,
        self_pid: Option<u32>,
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let ctx: &mut EnumCtx = &mut *(lparam.0 as *mut EnumCtx);

        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }

        // Skip tool windows (e.g. Win11 Search, Cortana popups, etc.)
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
            return BOOL(1);
        }

        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return BOOL(1);
        }
        let mut buf = vec![0u16; (len + 1) as usize];
        let copied = GetWindowTextW(hwnd, &mut buf);
        if copied <= 0 {
            return BOOL(1);
        }
        let title = String::from_utf16_lossy(&buf[..copied as usize]);

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if Some(pid) == ctx.self_pid {
            return BOOL(1);
        }

        let class_name = read_class_name(hwnd).unwrap_or_default();
        let exe_path = process_image_path(pid).unwrap_or_default();
        // Filter ProjectHub by exe filename as a backstop.
        if exe_filename(&exe_path).eq_ignore_ascii_case("projecthub.exe") {
            return BOOL(1);
        }

        let minimized = IsIconic(hwnd).as_bool();

        ctx.out.push(EnumeratedWindow {
            hwnd: hwnd.0 as isize,
            title,
            exe_path,
            class_name,
            pid,
            minimized,
        });

        BOOL(1)
    }

    unsafe fn read_class_name(hwnd: HWND) -> Option<String> {
        let mut buf = [0u16; 256];
        let copied = GetClassNameW(hwnd, &mut buf);
        if copied <= 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buf[..copied as usize]))
    }

    fn exe_filename(path: &str) -> String {
        PathBuf::from(path)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    fn process_image_path(pid: u32) -> Option<String> {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buf = vec![0u16; MAX_PATH as usize];
            let mut size = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_FORMAT(0),
                PWSTR(buf.as_mut_ptr()),
                &mut size,
            );
            let _ = CloseHandle(handle);
            if ok.is_err() {
                return None;
            }
            Some(String::from_utf16_lossy(&buf[..size as usize]))
        }
    }

    /// Restore (un-minimize) `hwnd` and bring it to the foreground.
    /// Uses the `AttachThreadInput` trick to bypass Windows foreground-lock
    /// when we don't own the recently-active input.
    pub fn focus_window(hwnd: isize) {
        unsafe {
            let target = HWND(hwnd as *mut _);
            // `IsIconic` -> SW_RESTORE; otherwise SW_SHOWNA so we don't steal
            // input twice.
            if IsIconic(target).as_bool() {
                let _ = ShowWindow(target, SW_RESTORE);
            } else {
                let _ = ShowWindow(target, SW_SHOWNA);
            }
            if SetForegroundWindow(target).as_bool() {
                return;
            }

            // Foreground-lock fallback: attach to the thread owning the
            // current foreground window, set foreground, detach.
            let foreground = GetForegroundWindow();
            if foreground.is_invalid() || foreground == target {
                return;
            }
            let mut fg_pid = 0u32;
            let fg_thread = GetWindowThreadProcessId(foreground, Some(&mut fg_pid));
            let mut tg_pid = 0u32;
            let tg_thread = GetWindowThreadProcessId(target, Some(&mut tg_pid));
            if fg_thread == 0 || tg_thread == 0 {
                return;
            }
            let _ = AttachThreadInput(fg_thread, tg_thread, true);
            let _ = SetForegroundWindow(target);
            let _ = AttachThreadInput(fg_thread, tg_thread, false);
        }
    }

    pub fn minimize_window(hwnd: isize) {
        unsafe {
            let _ = ShowWindow(HWND(hwnd as *mut _), SW_MINIMIZE);
        }
    }

    #[allow(dead_code)]
    pub fn restore_window(hwnd: isize) {
        unsafe {
            let target = HWND(hwnd as *mut _);
            if IsIconic(target).as_bool() {
                let _ = ShowWindow(target, SW_RESTORE);
            }
        }
    }

    pub fn current_pid() -> u32 {
        std::process::id()
    }
}

// ------------------------------------------------------------------
// Non-Windows stub
// ------------------------------------------------------------------
#[cfg(not(windows))]
mod stub_impl {
    use super::EnumeratedWindow;

    pub fn enumerate_windows(_self_pid: Option<u32>) -> Vec<EnumeratedWindow> {
        Vec::new()
    }
    pub fn focus_window(_hwnd: isize) {}
    pub fn minimize_window(_hwnd: isize) {}
    #[allow(dead_code)]
    pub fn restore_window(_hwnd: isize) {}
    pub fn current_pid() -> u32 {
        std::process::id()
    }
}

/// Match a stored window reference against the currently enumerated set.
///
/// Strategy (in order):
/// 1. Exact title match within the same exe.
/// 2. Substring match on the stored `title_pattern` within the same exe.
/// 3. Class-name + exe match (last resort — many windows share class names).
///
/// Returns `Some(EnumeratedWindow)` on success.
pub fn match_window<'a>(
    stored: &crate::project::WindowRef,
    candidates: &'a [EnumeratedWindow],
) -> Option<&'a EnumeratedWindow> {
    let exe_eq = |w: &&EnumeratedWindow| {
        path_eq_ignore_case(&w.exe_path, &stored.exe_path) || w.exe_path == stored.exe_path
    };

    if let Some(w) = candidates
        .iter()
        .filter(exe_eq)
        .find(|w| w.title == stored.title_snapshot)
    {
        return Some(w);
    }

    let needle = stored.title_pattern.trim().to_lowercase();
    if !needle.is_empty() {
        if let Some(w) = candidates
            .iter()
            .filter(exe_eq)
            .find(|w| w.title.to_lowercase().contains(&needle))
        {
            return Some(w);
        }
    }

    if let Some(class) = &stored.class_name {
        if !class.is_empty() {
            if let Some(w) = candidates
                .iter()
                .filter(exe_eq)
                .find(|w| &w.class_name == class)
            {
                return Some(w);
            }
        }
    }

    None
}

fn path_eq_ignore_case(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}
