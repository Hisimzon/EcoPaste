use super::{
    clear_low_resource_clipboard_state,
    consume_low_resource_clipboard_dirty_flag, consume_window_just_created, ensure_window_by_label,
    is_main_window, is_window_page_loaded, mark_window_pending_show,
    set_low_resource_clipboard_shortcut, take_low_resource_clipboard_queue, LOW_RESOURCE_MODE,
    MAIN_WINDOW_LABEL,
    PREFERENCE_WINDOW_LABEL,
};
use std::{
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    thread,
    time::Duration,
};
use tauri::{command, AppHandle, Emitter, Manager, Runtime, WebviewWindow};

const LISTEN_KEY_SHOW_WINDOW: &str = "show-window"; // 前端监听窗口显示事件，用于同步位置等逻辑

// 搜索模式标志（保留用于兼容，但 Windows 不抢占焦点模式下不使用）
pub static SEARCH_MODE: AtomicBool = AtomicBool::new(false);

// 输入模式标志：当为 true 时，不拦截键盘输入（用于 Modal 输入框等场景）
pub static INPUT_MODE: AtomicBool = AtomicBool::new(false);
pub static PINNED: AtomicBool = AtomicBool::new(false);
pub static MAIN_WINDOW_VISIBLE: AtomicBool = AtomicBool::new(false);
static MAIN_WINDOW_DESTROY_SEQ: AtomicU64 = AtomicU64::new(0);
pub const LOW_RESOURCE_DESTROY_DELAY_MS: u64 = 8000;

pub fn cancel_main_window_destroy() {
    MAIN_WINDOW_DESTROY_SEQ.fetch_add(1, Ordering::Relaxed);
}

pub fn schedule_main_window_destroy<R: Runtime>(app_handle: &AppHandle<R>, delay_ms: u64) {
    let seq = MAIN_WINDOW_DESTROY_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    let app_handle_clone = app_handle.clone();

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(delay_ms));

        if MAIN_WINDOW_DESTROY_SEQ.load(Ordering::Relaxed) != seq {
            return;
        }

        if !LOW_RESOURCE_MODE.load(Ordering::Relaxed) {
            return;
        }

        if MAIN_WINDOW_VISIBLE.load(Ordering::Relaxed) {
            return;
        }

        if let Some(window) = app_handle_clone.get_webview_window(MAIN_WINDOW_LABEL) {
            let _ = window.destroy();
        }
    });
}

pub fn show_window_now<R: Runtime>(window: &WebviewWindow<R>) {
    if is_main_window(window) {
        cancel_main_window_destroy();
        MAIN_WINDOW_VISIBLE.store(true, Ordering::Relaxed);
        // Ensure the main window does not steal focus when shown.
        let _ = window.set_focusable(false);
        let _ = window.show();
        let _ = window.unminimize();
        // 触发前端同步窗口位置（低占用唤醒场景）
        let _ = window.emit(LISTEN_KEY_SHOW_WINDOW, true);
    } else {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

// 显示窗口
#[command]
pub async fn show_window<R: Runtime>(
    app_handle: AppHandle<R>,
    window: WebviewWindow<R>,
    label: Option<String>,
) {
    let target_label = label
        .as_deref()
        .unwrap_or_else(|| window.label())
        .to_string();

    let window = if let Some(label) = label {
        ensure_window_by_label(&app_handle, &label).unwrap_or(window)
    } else {
        window
    };

    if consume_window_just_created(&target_label) && !is_window_page_loaded(&target_label) {
        mark_window_pending_show(&target_label);

        return;
    }

    show_window_now(&window);
}

// 进入搜索模式（保留用于兼容）
#[command]
pub async fn enter_search_mode<R: Runtime>(_app_handle: AppHandle<R>, _window: WebviewWindow<R>) {
    // Windows 不抢占焦点模式下不使用此功能
}

// 退出搜索模式（保留用于兼容）
#[command]
pub async fn exit_search_mode<R: Runtime>(_app_handle: AppHandle<R>, _window: WebviewWindow<R>) {
    // Windows 不抢占焦点模式下不使用此功能
}

// 进入输入模式（禁用键盘拦截，让窗口可聚焦，用于 Modal 输入框）
#[command]
pub async fn enter_input_mode<R: Runtime>(_app_handle: AppHandle<R>, window: WebviewWindow<R>) {
    INPUT_MODE.store(true, Ordering::Relaxed);

    if is_main_window(&window) {
        let _ = window.set_focusable(true);
        let _ = window.set_focus();
    }
}

// 退出输入模式（恢复键盘拦截，恢复窗口不可聚焦）
#[command]
pub async fn exit_input_mode<R: Runtime>(_app_handle: AppHandle<R>, window: WebviewWindow<R>) {
    INPUT_MODE.store(false, Ordering::Relaxed);

    if is_main_window(&window) {
        let _ = window.set_focusable(false);
    }
}

// 隐藏窗口
#[command]
pub async fn hide_window<R: Runtime>(app_handle: AppHandle<R>, window: WebviewWindow<R>) {
    if is_main_window(&window) {
        MAIN_WINDOW_VISIBLE.store(false, Ordering::Relaxed);
    }

    if LOW_RESOURCE_MODE.load(Ordering::Relaxed) {
        if is_main_window(&window) {
            let _ = window.hide();
            schedule_main_window_destroy(&app_handle, LOW_RESOURCE_DESTROY_DELAY_MS);
        } else {
            let _ = window.hide();
        }

        return;
    }

    let _ = window.hide();
}

// 设置低占用模式
#[command]
pub async fn set_low_resource_mode<R: Runtime>(
    app_handle: AppHandle<R>,
    _window: WebviewWindow<R>,
    enabled: bool,
    clipboard_shortcut: Option<String>,
) {
    let previous = LOW_RESOURCE_MODE.swap(enabled, Ordering::Relaxed);
    set_low_resource_clipboard_shortcut(clipboard_shortcut);

    if !enabled {
        cancel_main_window_destroy();
        clear_low_resource_clipboard_state();
        return;
    }

    if previous {
        return;
    }

    MAIN_WINDOW_VISIBLE.store(false, Ordering::Relaxed);
    INPUT_MODE.store(false, Ordering::Relaxed);

    if let Some(window) = app_handle.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.destroy();
    }

    if let Some(window) = app_handle.get_webview_window(PREFERENCE_WINDOW_LABEL) {
        let _ = window.destroy();
    }
}

#[command]
pub async fn consume_low_resource_clipboard_dirty<R: Runtime>(
    _app_handle: AppHandle<R>,
    _window: WebviewWindow<R>,
) -> bool {
    consume_low_resource_clipboard_dirty_flag()
}

#[command]
pub async fn drain_low_resource_clipboard_queue<R: Runtime>(
    _app_handle: AppHandle<R>,
    _window: WebviewWindow<R>,
) -> Vec<serde_json::Value> {
    take_low_resource_clipboard_queue()
}

// 显示任务栏图标
#[command]
pub async fn show_taskbar_icon<R: Runtime>(
    _app_handle: AppHandle<R>,
    window: WebviewWindow<R>,
    visible: bool,
) {
    let _ = window.set_skip_taskbar(!visible);
}

//置顶
#[command]
pub async fn set_pinned<R: Runtime>(
    _app_handle: AppHandle<R>,
    _window: WebviewWindow<R>,
    pinned: bool,
) {
    PINNED.store(pinned, Ordering::Relaxed);
}
