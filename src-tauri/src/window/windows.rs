//! Windows 窗口管理：剪贴板窗口始终不可聚焦，避免破坏外部粘贴目标。
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering};
use tauri::AppHandle;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowLongPtrW, IsWindow, SetForegroundWindow, SetWindowLongPtrW,
    SetWindowPos, ShowWindow, GWL_EXSTYLE, HWND_TOPMOST, SWP_FRAMECHANGED, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SW_HIDE, SW_SHOWNOACTIVATE, WS_EX_NOACTIVATE,
};

use super::{get_window, CLIPBOARD_WINDOW_LABEL};
use crate::core::Result;
use crate::{keyboard, mouse};

static CLIPBOARD_PASTE_TARGET: AtomicIsize = AtomicIsize::new(0);
static CLIPBOARD_WINDOW_VISIBLE: AtomicBool = AtomicBool::new(false);
static CLIPBOARD_VISIBILITY_REQUEST: AtomicU64 = AtomicU64::new(0);

/// 返回最新请求的剪贴板窗口可见状态，避免异步 WebView 显隐任务尚未执行时 toggle 误判。
pub fn is_clipboard_window_visible(_app_handle: &AppHandle) -> bool {
    CLIPBOARD_WINDOW_VISIBLE.load(Ordering::Acquire)
}

pub fn show_window(app_handle: &AppHandle, label: &str) -> Result<()> {
    let window = get_window(app_handle, label)?;
    if label == CLIPBOARD_WINDOW_LABEL {
        remember_clipboard_paste_target(&window)?;
        apply_no_activate(&window, true)?;
        show_without_activation(app_handle, &window)?;
    } else {
        window.show().map_err(|e| anyhow::anyhow!(e))?;
        window.unminimize().map_err(|e| anyhow::anyhow!(e))?;
        window.set_focus().map_err(|e| anyhow::anyhow!(e))?;
    }

    Ok(())
}

pub fn set_clipboard_window_editing(app_handle: &AppHandle, editing: bool) -> Result<()> {
    let window = get_window(app_handle, CLIPBOARD_WINDOW_LABEL)?;

    if editing {
        remember_clipboard_paste_target(&window)?;
    }

    // 窗口显隐由原生 ShowWindow 管理，tao 的内部 VISIBLE flag 不会同步。
    // 此处若调用 set_focusable，tao 会因内部仍是隐藏态而执行 SW_HIDE。
    apply_no_activate(&window, !editing)?;

    if editing {
        let hwnd = window_hwnd(&window)?;
        let foreground_hwnd = unsafe { GetForegroundWindow() };
        if foreground_hwnd != hwnd && !unsafe { SetForegroundWindow(hwnd) }.as_bool() {
            apply_no_activate(&window, true)?;
            return Err(anyhow::anyhow!("activate clipboard window for editing").into());
        }

        keyboard::disable_navigation_keys();
        return Ok(());
    }

    if is_clipboard_window_visible(app_handle) {
        keyboard::enable_navigation_keys(app_handle);
        mouse::enable_outside_click_hide(app_handle);
    }

    Ok(())
}

pub fn hide_window(app_handle: &AppHandle, label: &str) -> Result<()> {
    let window = get_window(app_handle, label)?;
    if label == CLIPBOARD_WINDOW_LABEL {
        hide_without_activation(app_handle, &window)?;
        if let Err(err) = apply_no_activate(&window, true) {
            log::warn!("reset clipboard window no-activate style on hide failed: {err:?}");
        }
    } else {
        window.hide().map_err(|e| anyhow::anyhow!(e))?;
    }

    Ok(())
}

/// EcoPaste 曾因手动编辑进入前台时，粘贴前恢复原外部目标窗口。
pub fn restore_clipboard_paste_target(app_handle: &AppHandle) -> Result<()> {
    let window = get_window(app_handle, CLIPBOARD_WINDOW_LABEL)?;
    let clipboard_hwnd = window_hwnd(&window)?;
    let foreground_hwnd = unsafe { GetForegroundWindow() };

    if foreground_hwnd.0 != 0 && foreground_hwnd != clipboard_hwnd {
        CLIPBOARD_PASTE_TARGET.store(foreground_hwnd.0, Ordering::Relaxed);
        return Ok(());
    }

    let target_hwnd = HWND(CLIPBOARD_PASTE_TARGET.load(Ordering::Relaxed));
    if target_hwnd.0 == 0
        || target_hwnd == clipboard_hwnd
        || !unsafe { IsWindow(target_hwnd) }.as_bool()
    {
        return Ok(());
    }

    if !unsafe { SetForegroundWindow(target_hwnd) }.as_bool() {
        return Err(anyhow::anyhow!("restore clipboard paste target window").into());
    }

    Ok(())
}

fn remember_clipboard_paste_target(window: &tauri::WebviewWindow) -> Result<()> {
    let clipboard_hwnd = window_hwnd(window)?;
    let foreground_hwnd = unsafe { GetForegroundWindow() };

    if foreground_hwnd.0 != 0 && foreground_hwnd != clipboard_hwnd {
        CLIPBOARD_PASTE_TARGET.store(foreground_hwnd.0, Ordering::Relaxed);
    }

    Ok(())
}

fn window_hwnd(window: &tauri::WebviewWindow) -> Result<HWND> {
    let raw_hwnd = window.hwnd().map_err(|e| anyhow::anyhow!(e))?;
    Ok(HWND(raw_hwnd.0 as isize))
}

fn apply_no_activate(window: &tauri::WebviewWindow, no_activate: bool) -> Result<()> {
    let hwnd = window_hwnd(window)?;

    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let no_activate_style = WS_EX_NOACTIVATE.0 as isize;
        let next_style = if no_activate {
            style | no_activate_style
        } else {
            style & !no_activate_style
        };
        if next_style != style {
            let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next_style);
        }

        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )
        .map_err(|e| anyhow::anyhow!(e))?;
    }

    Ok(())
}

/// 在无激活样式已经生效后显示主窗口，并同步 WebView2 可见状态，避免快捷键呼出期间抢占前台窗口。
fn show_without_activation(app_handle: &AppHandle, window: &tauri::WebviewWindow) -> Result<()> {
    let hwnd = window_hwnd(window)?;

    set_clipboard_visibility_on_ui_thread(app_handle, window, hwnd, true)
}

fn hide_without_activation(app_handle: &AppHandle, window: &tauri::WebviewWindow) -> Result<()> {
    let hwnd = window_hwnd(window)?;

    set_clipboard_visibility_on_ui_thread(app_handle, window, hwnd, false)
}

/// `with_webview` 可能把 closure 排队到 Tauri UI event loop。快捷键和托盘回调也可能运行
/// 在该线程，故这里只提交显隐任务，绝不能同步等待 closure，否则主线程会等待自身而死锁。
fn set_clipboard_visibility_on_ui_thread(
    app_handle: &AppHandle,
    window: &tauri::WebviewWindow,
    hwnd: HWND,
    visible: bool,
) -> Result<()> {
    let request = CLIPBOARD_VISIBILITY_REQUEST.fetch_add(1, Ordering::AcqRel) + 1;
    CLIPBOARD_WINDOW_VISIBLE.store(visible, Ordering::Release);

    if !visible {
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
        keyboard::disable_navigation_keys();
        mouse::disable_outside_click_hide();
    }

    let app_handle = app_handle.clone();

    let schedule_result = window.with_webview(move |webview| unsafe {
        if CLIPBOARD_VISIBILITY_REQUEST.load(Ordering::Acquire) != request {
            return;
        }

        if CLIPBOARD_WINDOW_VISIBLE.load(Ordering::Acquire) {
            if let Err(err) = webview.controller().SetIsVisible(true) {
                log::warn!("show clipboard webview controller failed: {err:?}");
            }

            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            keyboard::enable_navigation_keys(&app_handle);
            mouse::enable_outside_click_hide(&app_handle);
        } else if let Err(err) = webview.controller().SetIsVisible(false) {
            log::warn!("hide clipboard webview controller failed after native hide: {err:?}");
        }
    });

    if let Err(err) = schedule_result {
        if !visible {
            log::warn!("sync hidden clipboard webview state failed: {err:?}");
            return Ok(());
        }

        if CLIPBOARD_VISIBILITY_REQUEST.load(Ordering::Acquire) == request {
            CLIPBOARD_WINDOW_VISIBLE.store(false, Ordering::Release);
        }
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
        keyboard::disable_navigation_keys();
        mouse::disable_outside_click_hide();
        return Err(anyhow::anyhow!(err).into());
    }

    Ok(())
}

pub fn show_taskbar_icon(app_handle: &AppHandle, visible: bool) -> Result<()> {
    let window = get_window(app_handle, CLIPBOARD_WINDOW_LABEL)?;
    window
        .set_skip_taskbar(!visible)
        .map_err(|e| anyhow::anyhow!(e))?;
    Ok(())
}
