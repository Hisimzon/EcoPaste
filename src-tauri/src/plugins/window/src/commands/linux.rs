use tauri::{command, AppHandle, Runtime, WebviewWindow};

// 显示窗口
#[command]
pub async fn show_window<R: Runtime>(_app_handle: AppHandle<R>, window: WebviewWindow<R>) {
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
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
