use super::is_main_window;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{command, AppHandle, Manager, Runtime, WebviewWindow};

// 搜索模式标志（保留用于兼容，但 Windows 不抢占焦点模式下不使用）
pub static SEARCH_MODE: AtomicBool = AtomicBool::new(false);

// 输入模式标志：当为 true 时，不拦截键盘输入（用于 Modal 输入框等场景）
pub static INPUT_MODE: AtomicBool = AtomicBool::new(false);

// 显示窗口
#[command]
pub async fn show_window<R: Runtime>(
    app_handle: AppHandle<R>,
    window: WebviewWindow<R>,
    label: Option<String>,
) {
    let window = if let Some(label) = label {
        app_handle
            .get_webview_window(&label)
            .unwrap_or(window)
    } else {
        window
    };

    let _ = window.show();
    let _ = window.unminimize();

    if is_main_window(&window) {
        let _ = window.set_focusable(false);
    } else {
        let _ = window.set_focus();
    }
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
pub async fn hide_window<R: Runtime>(_app_handle: AppHandle<R>, window: WebviewWindow<R>) {
    let _ = window.hide();
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
