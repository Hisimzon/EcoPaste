use super::{
    clear_low_resource_clipboard_pending_state, ensure_window_by_label,
    set_low_resource_clipboard_shortcut, LOW_RESOURCE_MODE,
};
use std::sync::atomic::Ordering;
use tauri::{command, AppHandle, Manager, Runtime, WebviewWindow};

// 显示窗口
#[command]
pub async fn show_window<R: Runtime>(
    app_handle: AppHandle<R>,
    window: WebviewWindow<R>,
    label: Option<String>,
) {
    let window = if let Some(label) = label {
        ensure_window_by_label(&app_handle, &label).unwrap_or(window)
    } else {
        window
    };

    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

// 隐藏窗口
#[command]
pub async fn hide_window<R: Runtime>(_app_handle: AppHandle<R>, window: WebviewWindow<R>) {
    if LOW_RESOURCE_MODE.load(Ordering::Relaxed) {
        let _ = window.destroy();

        return;
    }

    let _ = window.hide();
}

// 设置低占用模式（Linux 暂仅记录状态，窗口唤醒仍由常规路径处理）
#[command]
pub async fn set_low_resource_mode<R: Runtime>(
    _app_handle: AppHandle<R>,
    _window: WebviewWindow<R>,
    enabled: bool,
    clipboard_shortcut: Option<String>,
) {
    LOW_RESOURCE_MODE.store(enabled, Ordering::Relaxed);
    set_low_resource_clipboard_shortcut(clipboard_shortcut);
}

#[command]
pub async fn consume_low_resource_clipboard_dirty<R: Runtime>(
    _app_handle: AppHandle<R>,
    _window: WebviewWindow<R>,
) -> bool {
    false
}

#[command]
pub async fn drain_low_resource_clipboard_queue<R: Runtime>(
    _app_handle: AppHandle<R>,
    _window: WebviewWindow<R>,
) -> Vec<serde_json::Value> {
    Vec::new()
}

#[command]
pub async fn clear_low_resource_clipboard_state<R: Runtime>(
    _app_handle: AppHandle<R>,
    _window: WebviewWindow<R>,
) {
    clear_low_resource_clipboard_pending_state();
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

// 进入搜索模式（Linux 暂不需要特殊处理）
#[command]
pub async fn enter_search_mode<R: Runtime>(_app_handle: AppHandle<R>, _window: WebviewWindow<R>) {}

// 退出搜索模式（Linux 暂不需要特殊处理）
#[command]
pub async fn exit_search_mode<R: Runtime>(_app_handle: AppHandle<R>, _window: WebviewWindow<R>) {}

// 进入输入模式（Linux 暂不需要特殊处理）
#[command]
pub async fn enter_input_mode<R: Runtime>(_app_handle: AppHandle<R>, _window: WebviewWindow<R>) {}

// 退出输入模式（Linux 暂不需要特殊处理）
#[command]
pub async fn exit_input_mode<R: Runtime>(_app_handle: AppHandle<R>, _window: WebviewWindow<R>) {}

//置顶
#[command]
pub async fn set_pinned<R: Runtime>(
    _app_handle: AppHandle<R>,
    _window: WebviewWindow<R>,
    _pinned: bool,
) {
}
