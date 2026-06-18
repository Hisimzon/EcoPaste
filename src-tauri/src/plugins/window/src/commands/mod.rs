use std::{
    collections::{HashSet, VecDeque},
    fs::read_to_string,
    sync::{
        atomic::{AtomicBool, Ordering},
        LazyLock, Mutex,
    },
};
use tauri::{
    async_runtime::spawn, webview::PageLoadEvent, window::Color, AppHandle, Manager, Runtime,
    WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

// 主窗口的label
pub static MAIN_WINDOW_LABEL: &str = "main";
// 偏好设置窗口的label
pub static PREFERENCE_WINDOW_LABEL: &str = "preference";
// 主窗口的title
pub static MAIN_WINDOW_TITLE: &str = "EcoPaste";

// 是否开启低占用模式
pub static LOW_RESOURCE_MODE: AtomicBool = AtomicBool::new(false);

pub static LOW_RESOURCE_CLIPBOARD_DIRTY: AtomicBool = AtomicBool::new(false);

const LOW_RESOURCE_CLIPBOARD_QUEUE_MAX: usize = 256;

static LOW_RESOURCE_CLIPBOARD_QUEUE: LazyLock<Mutex<VecDeque<serde_json::Value>>> =
    LazyLock::new(|| Mutex::new(VecDeque::new()));

static JUST_CREATED_WINDOW_LABELS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));
static PENDING_SHOW_WINDOW_LABELS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));
static PAGE_LOADED_WINDOW_LABELS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

// 低占用模式下用于唤醒主窗口的快捷键
static LOW_RESOURCE_CLIPBOARD_SHORTCUT: Mutex<Option<String>> = Mutex::new(None);

struct SavedWindowState {
    x: Option<f64>,
    y: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
}

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(target_os = "linux")]
pub use linux::*;

// 是否为主窗口
pub fn is_main_window<R: Runtime>(window: &WebviewWindow<R>) -> bool {
    window.label() == MAIN_WINDOW_LABEL
}

pub fn set_low_resource_clipboard_shortcut(shortcut: Option<String>) {
    let Ok(mut value) = LOW_RESOURCE_CLIPBOARD_SHORTCUT.lock() else {
        return;
    };

    *value = shortcut
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty());
}

pub fn get_low_resource_clipboard_shortcut() -> String {
    LOW_RESOURCE_CLIPBOARD_SHORTCUT
        .lock()
        .ok()
        .and_then(|value| value.clone())
        .unwrap_or_else(|| "Alt+C".to_string())
}

pub fn mark_low_resource_clipboard_dirty() {
    LOW_RESOURCE_CLIPBOARD_DIRTY.store(true, Ordering::Relaxed);
}

pub fn consume_low_resource_clipboard_dirty_flag() -> bool {
    LOW_RESOURCE_CLIPBOARD_DIRTY.swap(false, Ordering::Relaxed)
}

pub fn push_low_resource_clipboard_snapshot(snapshot: serde_json::Value) {
    if snapshot.is_null() {
        return;
    }

    let Ok(mut queue) = LOW_RESOURCE_CLIPBOARD_QUEUE.lock() else {
        mark_low_resource_clipboard_dirty();
        return;
    };

    if queue.back().is_some_and(|last| last == &snapshot) {
        return;
    }

    queue.push_back(snapshot);

    while queue.len() > LOW_RESOURCE_CLIPBOARD_QUEUE_MAX {
        queue.pop_front();
    }
}

pub fn take_low_resource_clipboard_queue() -> Vec<serde_json::Value> {
    LOW_RESOURCE_CLIPBOARD_QUEUE
        .lock()
        .ok()
        .map(|mut queue| queue.drain(..).collect())
        .unwrap_or_default()
}

pub fn clear_low_resource_clipboard_pending_state() {
    LOW_RESOURCE_CLIPBOARD_DIRTY.store(false, Ordering::Relaxed);

    if let Ok(mut queue) = LOW_RESOURCE_CLIPBOARD_QUEUE.lock() {
        queue.clear();
    }
}

fn mark_window_just_created(label: &str) {
    if let Ok(mut labels) = JUST_CREATED_WINDOW_LABELS.lock() {
        labels.insert(label.to_string());
    }

    if let Ok(mut labels) = PAGE_LOADED_WINDOW_LABELS.lock() {
        labels.remove(label);
    }
}

pub fn consume_window_just_created(label: &str) -> bool {
    JUST_CREATED_WINDOW_LABELS
        .lock()
        .ok()
        .map(|mut labels| labels.remove(label))
        .unwrap_or(false)
}

pub fn mark_window_pending_show(label: &str) {
    if let Ok(mut labels) = PENDING_SHOW_WINDOW_LABELS.lock() {
        labels.insert(label.to_string());
    }
}

fn consume_window_pending_show(label: &str) -> bool {
    PENDING_SHOW_WINDOW_LABELS
        .lock()
        .ok()
        .map(|mut labels| labels.remove(label))
        .unwrap_or(false)
}

fn mark_window_page_loaded(label: &str) {
    if let Ok(mut labels) = PAGE_LOADED_WINDOW_LABELS.lock() {
        labels.insert(label.to_string());
    }
}

pub fn is_window_page_loaded(label: &str) -> bool {
    PAGE_LOADED_WINDOW_LABELS
        .lock()
        .ok()
        .map(|labels| labels.contains(label))
        .unwrap_or(false)
}

fn on_window_page_loaded<R: Runtime>(window: WebviewWindow<R>) {
    let label = window.label().to_string();
    mark_window_page_loaded(&label);

    if !consume_window_pending_show(&label) {
        return;
    }

    #[cfg(target_os = "windows")]
    {
        windows::show_window_now(&window);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn with_page_load_handler<R: Runtime>(
    builder: WebviewWindowBuilder<R, AppHandle<R>>,
) -> WebviewWindowBuilder<R, AppHandle<R>> {
    builder.on_page_load(|window, payload| {
        if payload.event() != PageLoadEvent::Finished {
            return;
        }

        on_window_page_loaded(window);
    })
}

fn create_main_window<R: Runtime>(app_handle: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    let mut builder = with_page_load_handler(WebviewWindowBuilder::new(
        app_handle,
        MAIN_WINDOW_LABEL,
        WebviewUrl::App("index.html".into()),
    ))
    .title(MAIN_WINDOW_TITLE)
    .visible(false)
    .always_on_top(true)
    .shadow(true)
    .decorations(false)
    .transparent(true)
    .background_color(Color(24, 24, 27, 0))
    .skip_taskbar(true)
    .inner_size(360.0, 600.0)
    .min_inner_size(360.0, 600.0);

    if let Some(state) = read_saved_window_state(app_handle, MAIN_WINDOW_LABEL) {
        if let (Some(x), Some(y)) = (state.x, state.y) {
            builder = builder.position(x, y);
        } else {
            builder = builder.center();
        }

        if let (Some(width), Some(height)) = (state.width, state.height) {
            builder = builder.inner_size(width, height);
        }
    } else {
        builder = builder.center();
    }

    let window = builder.build().ok()?;
    mark_window_just_created(MAIN_WINDOW_LABEL);

    Some(window)
}

fn create_preference_window<R: Runtime>(app_handle: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    let mut builder = with_page_load_handler(WebviewWindowBuilder::new(
        app_handle,
        PREFERENCE_WINDOW_LABEL,
        WebviewUrl::App("index.html/#/preference".into()),
    ))
    .title("EcoPaste")
    .visible(false)
    .skip_taskbar(true)
    .inner_size(700.0, 480.0)
    .min_inner_size(700.0, 480.0);

    if let Some(state) = read_saved_window_state(app_handle, PREFERENCE_WINDOW_LABEL) {
        if let (Some(x), Some(y)) = (state.x, state.y) {
            builder = builder.position(x, y);
        } else {
            builder = builder.center();
        }

        if let (Some(width), Some(height)) = (state.width, state.height) {
            builder = builder.inner_size(width, height);
        }
    } else {
        builder = builder.center();
    }

    let window = builder.build().ok()?;
    mark_window_just_created(PREFERENCE_WINDOW_LABEL);

    Some(window)
}

fn read_saved_window_state<R: Runtime>(
    app_handle: &AppHandle<R>,
    label: &str,
) -> Option<SavedWindowState> {
    let app_data_dir = app_handle.path().app_data_dir().ok()?;
    let ext = if cfg!(debug_assertions) {
        "dev.json"
    } else {
        "json"
    };
    let state_path = app_data_dir.join(format!(".window-state.{ext}"));
    let content = read_to_string(state_path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;
    let window = value.get(label)?;

    Some(SavedWindowState {
        x: window.get("x").and_then(|item| item.as_f64()),
        y: window.get("y").and_then(|item| item.as_f64()),
        width: window.get("width").and_then(|item| item.as_f64()),
        height: window.get("height").and_then(|item| item.as_f64()),
    })
}

pub fn ensure_main_window<R: Runtime>(app_handle: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    if let Some(window) = app_handle.get_webview_window(MAIN_WINDOW_LABEL) {
        return Some(window);
    }

    create_main_window(app_handle)
}

pub fn ensure_window_by_label<R: Runtime>(
    app_handle: &AppHandle<R>,
    label: &str,
) -> Option<WebviewWindow<R>> {
    if label == MAIN_WINDOW_LABEL {
        return ensure_main_window(app_handle);
    }

    if label == PREFERENCE_WINDOW_LABEL {
        if let Some(window) = app_handle.get_webview_window(PREFERENCE_WINDOW_LABEL) {
            return Some(window);
        }

        return create_preference_window(app_handle);
    }

    app_handle.get_webview_window(label)
}

// 显示主窗口
pub fn show_main_window<R: Runtime>(app_handle: &AppHandle<R>) {
    show_window_by_label(app_handle, MAIN_WINDOW_LABEL);
}

// 显示偏好设置窗口
pub fn show_preference_window<R: Runtime>(app_handle: &AppHandle<R>) {
    show_window_by_label(app_handle, PREFERENCE_WINDOW_LABEL);
}

// 显示指定 label 的窗口
fn show_window_by_label<R: Runtime>(app_handle: &AppHandle<R>, label: &str) {
    let target_window = ensure_window_by_label(app_handle, label);

    if let Some(window) = target_window {
        let app_handle_clone = app_handle.clone();

        spawn(async move {
            show_window(app_handle_clone, window, None).await;
        });
    }
}
