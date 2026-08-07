use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use tauri::{AppHandle, Manager};
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::POINT;
use winapi::um::processthreadsapi::GetCurrentThreadId;
use winapi::um::winuser::{
    CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx, MSG,
    MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_QUIT, WM_RBUTTONDOWN,
};

use crate::window::{self, CLIPBOARD_WINDOW_LABEL};

static ENABLED: AtomicBool = AtomicBool::new(false);
static HOOK_RUNNING: AtomicBool = AtomicBool::new(false);
static HOOK_THREAD_ID: Mutex<Option<u32>> = Mutex::new(None);
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

pub fn enable_outside_click_hide(app: &AppHandle) {
    let _ = APP_HANDLE.set(app.clone());
    ENABLED.store(true, Ordering::Relaxed);
    start_hook_thread();
}

/// 鼠标 hook 同一时刻只允许一个线程；旧线程退出期间重新启用时，由退出路径补启动。
fn start_hook_thread() {
    if HOOK_RUNNING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    std::thread::spawn(|| unsafe {
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(hook_proc), null_mut(), 0);
        if hook.is_null() {
            log::error!("SetWindowsHookExW(WH_MOUSE_LL) failed");
            HOOK_RUNNING.store(false, Ordering::Release);
            return;
        }

        *HOOK_THREAD_ID.lock().expect("hook thread id poisoned") = Some(GetCurrentThreadId());

        let mut msg: MSG = std::mem::zeroed();
        if ENABLED.load(Ordering::Acquire) {
            // GetMessageW 收到 WM_QUIT 返回 0 → 消息泵自然退出。
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {}
        }

        UnhookWindowsHookEx(hook);
        *HOOK_THREAD_ID.lock().expect("hook thread id poisoned") = None;
        HOOK_RUNNING.store(false, Ordering::Release);

        if ENABLED.load(Ordering::Acquire) {
            start_hook_thread();
        }
    });
}

pub fn disable_outside_click_hide() {
    ENABLED.store(false, Ordering::Relaxed);

    let tid = *HOOK_THREAD_ID
        .lock()
        .expect("hook thread id poisoned");
    if let Some(tid) = tid {
        unsafe {
            PostThreadMessageW(tid, WM_QUIT, 0, 0);
        }
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 || !ENABLED.load(Ordering::Relaxed) {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }

    let msg = wparam as UINT;
    let is_button_down = matches!(msg, WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN);
    if !is_button_down {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }

    let data = &*(lparam as *const MSLLHOOKSTRUCT);
    let cursor = data.pt;

    if let Some(app) = APP_HANDLE.get() {
        if window::should_auto_hide_clipboard_window()
            && cursor_outside_clipboard_window(app, cursor)
        {
            schedule_hide(app);
        }
    }

    // 不吞键：用户的点击应该正常落到目标窗口。
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

fn cursor_outside_clipboard_window(app: &AppHandle, cursor: POINT) -> bool {
    let Some(window) = app.get_webview_window(CLIPBOARD_WINDOW_LABEL) else {
        return false;
    };
    if !window::is_clipboard_window_visible(app) {
        return false;
    }

    let Ok(position) = window.outer_position() else {
        return false;
    };
    let Ok(size) = window.outer_size() else {
        return false;
    };

    cursor.x < position.x
        || cursor.x >= position.x + size.width as i32
        || cursor.y < position.y
        || cursor.y >= position.y + size.height as i32
}

fn schedule_hide(app: &AppHandle) {
    let handle = app.clone();
    if let Err(err) = app.run_on_main_thread(move || {
        if !window::should_auto_hide_clipboard_window() {
            return;
        }

        if let Err(err) = window::hide_window(&handle, CLIPBOARD_WINDOW_LABEL) {
            log::warn!("auto-hide clipboard window failed: {err}");
        }
    }) {
        log::warn!("schedule auto-hide failed: {err}");
    }
}
