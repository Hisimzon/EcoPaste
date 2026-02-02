use super::is_main_window;
use std::sync::atomic::AtomicBool;
use tauri::{command, AppHandle, Runtime, WebviewWindow};

// 搜索模式标志（保留用于兼容，但 Windows 不抢占焦点模式下不使用）
pub static SEARCH_MODE: AtomicBool = AtomicBool::new(false);

// 显示窗口
#[command]
pub async fn show_window<R: Runtime>(_app_handle: AppHandle<R>, window: WebviewWindow<R>) {
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
