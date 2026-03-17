mod core;

use core::{prevent_default, setup};
use tauri::{generate_context, Builder, Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_eco_window::{
    show_main_window,
    show_preference_window,
    MAIN_WINDOW_LABEL,
    PREFERENCE_WINDOW_LABEL,
};
#[cfg(target_os = "windows")]
use tauri_plugin_opener::OpenerExt;
#[cfg(target_os = "windows")]
use tauri_plugin_eco_window::{
    mark_low_resource_clipboard_dirty, push_low_resource_clipboard_snapshot, LOW_RESOURCE_MODE,
    MAIN_WINDOW_VISIBLE,
};
use tauri_plugin_log::{Target, TargetKind};

#[cfg(target_os = "windows")]
use std::{
    fs::read_to_string,
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};
#[cfg(target_os = "windows")]
use tauri::Emitter;
#[cfg(target_os = "windows")]
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
#[cfg(target_os = "windows")]
use tauri::tray::TrayIconBuilder;

#[cfg(target_os = "windows")]
const TRAY_MENU_PREFERENCE_ID: &str = "tray.preference";
#[cfg(target_os = "windows")]
const TRAY_MENU_TOGGLE_LISTEN_ID: &str = "tray.toggle-listen";
#[cfg(target_os = "windows")]
const TRAY_MENU_CHECK_UPDATE_ID: &str = "tray.check-update";
#[cfg(target_os = "windows")]
const TRAY_MENU_OPEN_SOURCE_ID: &str = "tray.open-source";
#[cfg(target_os = "windows")]
const TRAY_MENU_RELAUNCH_ID: &str = "tray.relaunch";
#[cfg(target_os = "windows")]
const TRAY_MENU_EXIT_ID: &str = "tray.exit";
#[cfg(target_os = "windows")]
const TRAY_ID: &str = "app-tray";
#[cfg(target_os = "windows")]
const LISTEN_KEY_UPDATE_APP: &str = "update-app";
#[cfg(target_os = "windows")]
const LISTEN_KEY_STORE_CHANGED: &str = "store-changed";
#[cfg(target_os = "windows")]
const LISTEN_KEY_ALLOW_APP_EXIT: &str = "allow-app-exit";
#[cfg(target_os = "windows")]
const GITHUB_LINK: &str = "https://github.com/EcoPasteHub/EcoPaste";

#[cfg(target_os = "windows")]
static LOW_RESOURCE_TRAY_LISTEN_CLIPBOARD_ENABLED: AtomicBool = AtomicBool::new(true);
#[cfg(target_os = "windows")]
static ALLOW_LOW_RESOURCE_EXIT_REQUEST: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
enum TrayLocale {
    EnUs,
    ZhCn,
    ZhTw,
    JaJp,
}

#[cfg(target_os = "windows")]
struct TrayLabels {
    preference: &'static str,
    start_listening: &'static str,
    stop_listening: &'static str,
    check_update: &'static str,
    open_source: &'static str,
    version: &'static str,
    relaunch: &'static str,
    exit: &'static str,
}

#[cfg(target_os = "windows")]
impl TrayLocale {
    fn from_str(value: &str) -> Self {
        match value {
            "zh-CN" => Self::ZhCn,
            "zh-TW" => Self::ZhTw,
            "ja-JP" => Self::JaJp,
            _ => Self::EnUs,
        }
    }
}

#[cfg(target_os = "windows")]
fn read_current_language<R: tauri::Runtime>(app_handle: &tauri::AppHandle<R>) -> TrayLocale {
    let ext = if cfg!(debug_assertions) {
        "dev.json"
    } else {
        "json"
    };

    let Some(app_data_dir) = app_handle.path().app_data_dir().ok() else {
        return TrayLocale::EnUs;
    };

    let store_path = app_data_dir.join(format!(".store.{ext}"));
    let Ok(content) = read_to_string(store_path) else {
        return TrayLocale::EnUs;
    };

    let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
        return TrayLocale::EnUs;
    };

    json.get("globalStore")
        .and_then(|value| value.get("appearance"))
        .and_then(|value| value.get("language"))
        .and_then(|value| value.as_str())
        .map(TrayLocale::from_str)
        .unwrap_or(TrayLocale::EnUs)
}

#[cfg(target_os = "windows")]
fn read_show_menubar_icon<R: tauri::Runtime>(app_handle: &tauri::AppHandle<R>) -> bool {
    let ext = if cfg!(debug_assertions) {
        "dev.json"
    } else {
        "json"
    };

    let Some(app_data_dir) = app_handle.path().app_data_dir().ok() else {
        return true;
    };

    let store_path = app_data_dir.join(format!(".store.{ext}"));
    let Ok(content) = read_to_string(store_path) else {
        return true;
    };

    let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
        return true;
    };

    json.get("globalStore")
        .and_then(|value| value.get("app"))
        .and_then(|value| value.get("showMenubarIcon"))
        .and_then(|value| value.as_bool())
        .unwrap_or(true)
}

#[cfg(target_os = "windows")]
fn tray_labels(locale: TrayLocale) -> TrayLabels {
    match locale {
        TrayLocale::ZhCn => TrayLabels {
            preference: "偏好设置",
            start_listening: "开始监听",
            stop_listening: "停止监听",
            check_update: "检查更新",
            open_source: "打开源码地址",
            version: "版本",
            relaunch: "重启",
            exit: "退出",
        },
        TrayLocale::ZhTw => TrayLabels {
            preference: "偏好設定",
            start_listening: "開始監聽",
            stop_listening: "停止監聽",
            check_update: "檢查更新",
            open_source: "開啟原始碼位址",
            version: "版本",
            relaunch: "重新啟動",
            exit: "結束",
        },
        TrayLocale::JaJp => TrayLabels {
            preference: "環境設定",
            start_listening: "監視開始",
            stop_listening: "監視停止",
            check_update: "更新確認",
            open_source: "ソースを開く",
            version: "バージョン",
            relaunch: "再起動",
            exit: "終了",
        },
        TrayLocale::EnUs => TrayLabels {
            preference: "Preference",
            start_listening: "Start Listening",
            stop_listening: "Stop Listening",
            check_update: "Check Update",
            open_source: "Open Source Address",
            version: "Version",
            relaunch: "Relaunch",
            exit: "Exit",
        },
    }
}

#[cfg(target_os = "windows")]
fn get_low_resource_toggle_listen_text(enabled: bool, labels: &TrayLabels) -> &'static str {
    if enabled {
        labels.stop_listening
    } else {
        labels.start_listening
    }
}

#[cfg(target_os = "windows")]
fn sync_low_resource_tray_menu<R: tauri::Runtime>(app_handle: &tauri::AppHandle<R>) {
    let Some(tray) = app_handle.tray_by_id(TRAY_ID) else {
        return;
    };

    let listening_enabled = LOW_RESOURCE_TRAY_LISTEN_CLIPBOARD_ENABLED.load(Ordering::Relaxed);
    let labels = tray_labels(read_current_language(app_handle));

    let Ok(preference_item) = MenuItem::with_id(
        app_handle,
        TRAY_MENU_PREFERENCE_ID,
        labels.preference,
        true,
        None::<&str>,
    ) else {
        return;
    };

    let Ok(toggle_listen_item) = MenuItem::with_id(
        app_handle,
        TRAY_MENU_TOGGLE_LISTEN_ID,
        get_low_resource_toggle_listen_text(listening_enabled, &labels),
        true,
        None::<&str>,
    ) else {
        return;
    };

    let Ok(separator_1) = PredefinedMenuItem::separator(app_handle) else {
        return;
    };

    let Ok(check_update_item) = MenuItem::with_id(
        app_handle,
        TRAY_MENU_CHECK_UPDATE_ID,
        labels.check_update,
        true,
        None::<&str>,
    ) else {
        return;
    };

    let Ok(open_source_item) = MenuItem::with_id(
        app_handle,
        TRAY_MENU_OPEN_SOURCE_ID,
        labels.open_source,
        true,
        None::<&str>,
    ) else {
        return;
    };

    let Ok(separator_2) = PredefinedMenuItem::separator(app_handle) else {
        return;
    };

    let version_text = format!("{} v{}", labels.version, app_handle.package_info().version);
    let Ok(version_item) = MenuItem::new(app_handle, version_text, false, None::<&str>) else {
        return;
    };

    let Ok(relaunch_item) = MenuItem::with_id(
        app_handle,
        TRAY_MENU_RELAUNCH_ID,
        labels.relaunch,
        true,
        None::<&str>,
    ) else {
        return;
    };

    let Ok(exit_item) = MenuItem::with_id(
        app_handle,
        TRAY_MENU_EXIT_ID,
        labels.exit,
        true,
        None::<&str>,
    ) else {
        return;
    };

    let Ok(menu) = Menu::with_items(
        app_handle,
        &[
            &preference_item,
            &toggle_listen_item,
            &separator_1,
            &check_update_item,
            &open_source_item,
            &separator_2,
            &version_item,
            &relaunch_item,
            &exit_item,
        ],
    ) else {
        return;
    };

    let _ = tray.set_menu(Some(menu));
}

#[cfg(target_os = "windows")]
fn ensure_single_tray_icon<R: tauri::Runtime>(app_handle: &tauri::AppHandle<R>) {
    if app_handle.tray_by_id(TRAY_ID).is_none() {
        let Some(icon) = app_handle.default_window_icon().cloned() else {
            return;
        };

        let Ok(_) = TrayIconBuilder::with_id(TRAY_ID).icon(icon).build(app_handle) else {
            return;
        };
    }

    if let Some(tray) = app_handle.tray_by_id(TRAY_ID) {
        let _ = tray.set_visible(read_show_menubar_icon(app_handle));
    }

    sync_low_resource_tray_menu(app_handle);
}

#[cfg(target_os = "windows")]
async fn read_current_clipboard_snapshot<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
) -> Option<serde_json::Value> {
    let mut result = serde_json::Map::new();
    let mut text_count: Option<usize> = None;

    if matches!(tauri_plugin_clipboard_x::has_text().await, Ok(true)) {
        if let Ok(text) = tauri_plugin_clipboard_x::read_text().await {
            text_count = Some(text.len());
            result.insert(
                "text".to_string(),
                serde_json::json!({
                    "type": "text",
                    "value": text,
                    "count": text_count.unwrap_or(0),
                }),
            );
        }
    }

    if matches!(tauri_plugin_clipboard_x::has_rtf().await, Ok(true)) {
        if let Ok(rtf) = tauri_plugin_clipboard_x::read_rtf().await {
            result.insert(
                "rtf".to_string(),
                serde_json::json!({
                    "type": "rtf",
                    "value": rtf,
                    "count": text_count.unwrap_or(rtf.len()),
                }),
            );
        }
    }

    if matches!(tauri_plugin_clipboard_x::has_html().await, Ok(true)) {
        if let Ok(html) = tauri_plugin_clipboard_x::read_html().await {
            result.insert(
                "html".to_string(),
                serde_json::json!({
                    "type": "html",
                    "value": html,
                    "count": text_count.unwrap_or(html.len()),
                }),
            );
        }
    }

    if matches!(tauri_plugin_clipboard_x::has_image().await, Ok(true)) {
        if let Ok(image) = tauri_plugin_clipboard_x::read_image(app_handle.clone(), None).await {
            let path = image.path.to_string_lossy().to_string();

            result.insert(
                "image".to_string(),
                serde_json::json!({
                    "type": "image",
                    "value": path,
                    "count": image.size,
                    "width": image.width,
                    "height": image.height,
                }),
            );
        }
    }

    if matches!(tauri_plugin_clipboard_x::has_files().await, Ok(true)) {
        if let Ok(files) = tauri_plugin_clipboard_x::read_files().await {
            result.insert(
                "files".to_string(),
                serde_json::json!({
                    "type": "files",
                    "value": files.paths,
                    "count": files.size,
                }),
            );
        }
    }

    if result.is_empty() {
        return None;
    }

    Some(serde_json::Value::Object(result))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = Builder::default()
        .setup(|app| {
            let app_handle = app.handle();

            let main_window = app.get_webview_window(MAIN_WINDOW_LABEL).unwrap();

            let preference_window = app.get_webview_window(PREFERENCE_WINDOW_LABEL).unwrap();

            setup::default(&app_handle, main_window.clone(), preference_window.clone());

            #[cfg(target_os = "windows")]
            {
                use tauri::Listener;

                ensure_single_tray_icon(&app_handle);

                let app_handle_for_clipboard_event = app_handle.clone();

                app_handle.listen_any("plugin:clipboard-x://clipboard_changed", move |_| {
                    let app_handle_for_clipboard_capture = app_handle_for_clipboard_event.clone();

                    tauri::async_runtime::spawn(async move {
                        if !LOW_RESOURCE_MODE.load(Ordering::Relaxed) {
                            return;
                        }

                        if !LOW_RESOURCE_TRAY_LISTEN_CLIPBOARD_ENABLED.load(Ordering::Relaxed) {
                            return;
                        }

                        let main_window_missing = app_handle_for_clipboard_capture
                            .get_webview_window(MAIN_WINDOW_LABEL)
                            .is_none();

                        if !main_window_missing {
                            return;
                        }

                        if let Some(snapshot) =
                            read_current_clipboard_snapshot(app_handle_for_clipboard_capture).await
                        {
                            push_low_resource_clipboard_snapshot(snapshot);
                        } else {
                            mark_low_resource_clipboard_dirty();
                        }
                    });
                });

                let app_handle_for_store_changed = app_handle.clone();

                app_handle.listen_any(LISTEN_KEY_STORE_CHANGED, move |_| {
                    if !LOW_RESOURCE_MODE.load(Ordering::Relaxed) {
                        return;
                    }

                    sync_low_resource_tray_menu(&app_handle_for_store_changed);
                });

                app_handle.listen_any(LISTEN_KEY_ALLOW_APP_EXIT, move |_| {
                    ALLOW_LOW_RESOURCE_EXIT_REQUEST
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                    LOW_RESOURCE_MODE.store(false, std::sync::atomic::Ordering::SeqCst);
                });

                let app_handle_for_clipboard_watch = app_handle.clone();

                tauri::async_runtime::spawn(async move {
                    loop {
                        match tauri_plugin_clipboard_x::start_listening(
                            app_handle_for_clipboard_watch.clone(),
                        )
                        .await
                        {
                            Ok(_) => {
                                break;
                            }
                            Err(err) => {
                                eprintln!("clipboard listener error: {:?}", err);
                                thread::sleep(Duration::from_millis(1200));
                            }
                        }
                    }
                });
            }

            Ok(())
        })
        // 确保在 windows 和 linux 上只有一个 app 实例在运行：https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/single-instance
        .plugin(tauri_plugin_single_instance::init(
            |app_handle, _argv, _cwd| {
                show_main_window(app_handle);
            },
        ))
        // app 自启动：https://github.com/tauri-apps/tauri-plugin-autostart/tree/v2
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--auto-launch"]),
        ))
        // 数据库：https://github.com/tauri-apps/tauri-plugin-sql/tree/v2
        .plugin(tauri_plugin_sql::Builder::default().build())
        // 日志插件：https://github.com/tauri-apps/tauri-plugin-log/tree/v2
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: None }),
                    Target::new(TargetKind::Webview),
                ])
                .build(),
        )
        // 快捷键插件: https://github.com/tauri-apps/tauri-plugin-global-shortcut
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        // 操作系统相关信息插件：https://github.com/tauri-apps/tauri-plugin-os
        .plugin(tauri_plugin_os::init())
        // 系统级别对话框插件：https://github.com/tauri-apps/tauri-plugin-dialog
        .plugin(tauri_plugin_dialog::init())
        // 访问文件系统插件：https://github.com/tauri-apps/tauri-plugin-fs
        .plugin(tauri_plugin_fs::init())
        // 更新插件：https://github.com/tauri-apps/tauri-plugin-updater
        .plugin(tauri_plugin_updater::Builder::new().build())
        // 进程相关插件：https://github.com/tauri-apps/tauri-plugin-process
        .plugin(tauri_plugin_process::init())
        // 检查和请求 macos 系统权限：https://github.com/ayangweb/tauri-plugin-macos-permissions
        .plugin(tauri_plugin_macos_permissions::init())
        // 拓展了对文件和目录的操作：https://github.com/ayangweb/tauri-plugin-fs-pro
        .plugin(tauri_plugin_fs_pro::init())
        // 获取系统获取系统的区域设置：https://github.com/ayangweb/tauri-plugin-locale
        .plugin(tauri_plugin_locale::init())
        // 打开文件或者链接：https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/opener
        .plugin(tauri_plugin_opener::init())
        // 禁用 webview 的默认行为：https://github.com/ferreira-tb/tauri-plugin-prevent-default
        .plugin(prevent_default::init())
        // 剪贴板插件：https://github.com/ayangweb/tauri-plugin-clipboard-x
        .plugin(tauri_plugin_clipboard_x::init())
        // 自定义的窗口管理插件
        .plugin(tauri_plugin_eco_window::init())
        // 自定义粘贴的插件
        .plugin(tauri_plugin_eco_paste::init())
        // 自定义判断是否自动启动的插件
        .plugin(tauri_plugin_eco_autostart::init())
        .on_window_event(|window, event| match event {
            // 让 app 保持在后台运行：https://tauri.app/v1/guides/features/system-tray/#preventing-the-app-from-closing
            WindowEvent::CloseRequested { api, .. } => {
                #[cfg(target_os = "windows")]
                {
                    if LOW_RESOURCE_MODE.load(std::sync::atomic::Ordering::Relaxed)
                        && (window.label() == MAIN_WINDOW_LABEL
                            || window.label() == PREFERENCE_WINDOW_LABEL)
                    {
                        let _ = window.destroy();
                        api.prevent_close();

                        return;
                    }

                    if window.label() == MAIN_WINDOW_LABEL {
                        MAIN_WINDOW_VISIBLE.store(false, std::sync::atomic::Ordering::Relaxed);
                    }
                }

                window.hide().unwrap();

                api.prevent_close();
            }
            _ => {}
        })
        .build(generate_context!())
        .expect("error while running tauri application");

    app.run(|app_handle, event| match event {
        #[cfg(target_os = "windows")]
        tauri::RunEvent::ExitRequested { api, .. } => {
            if ALLOW_LOW_RESOURCE_EXIT_REQUEST.swap(false, std::sync::atomic::Ordering::SeqCst) {
                return;
            }

            if LOW_RESOURCE_MODE.load(std::sync::atomic::Ordering::SeqCst) {
                api.prevent_exit();
            }
        }
        #[cfg(target_os = "windows")]
        tauri::RunEvent::MenuEvent(menu_event) => {
            let menu_id = menu_event.id().as_ref();

            if menu_id == TRAY_MENU_TOGGLE_LISTEN_ID {
                let enabled = !LOW_RESOURCE_TRAY_LISTEN_CLIPBOARD_ENABLED
                    .load(std::sync::atomic::Ordering::Relaxed);

                LOW_RESOURCE_TRAY_LISTEN_CLIPBOARD_ENABLED
                    .store(enabled, std::sync::atomic::Ordering::Relaxed);

                if LOW_RESOURCE_MODE.load(std::sync::atomic::Ordering::Relaxed) {
                    sync_low_resource_tray_menu(app_handle);
                }

                return;
            }

            if !LOW_RESOURCE_MODE.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }

            match menu_id {
                TRAY_MENU_PREFERENCE_ID => {
                    show_preference_window(app_handle);
                }
                TRAY_MENU_CHECK_UPDATE_ID => {
                    show_preference_window(app_handle);

                    let app_handle_clone = app_handle.clone();

                    thread::spawn(move || {
                        for _ in 0..25 {
                            if tauri_plugin_eco_window::is_window_page_loaded(PREFERENCE_WINDOW_LABEL) {
                                break;
                            }

                            thread::sleep(Duration::from_millis(80));
                        }

                        let _ = app_handle_clone.emit_to(
                            PREFERENCE_WINDOW_LABEL,
                            LISTEN_KEY_UPDATE_APP,
                            true,
                        );
                    });
                }
                TRAY_MENU_OPEN_SOURCE_ID => {
                    if app_handle
                        .opener()
                        .open_url(GITHUB_LINK, None::<&str>)
                        .is_err()
                    {
                        let _ = Command::new("explorer").arg(GITHUB_LINK).spawn();
                    }
                }
                TRAY_MENU_RELAUNCH_ID => {
                    ALLOW_LOW_RESOURCE_EXIT_REQUEST
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                    LOW_RESOURCE_MODE.store(false, std::sync::atomic::Ordering::SeqCst);
                    app_handle.request_restart();
                }
                TRAY_MENU_EXIT_ID => {
                    ALLOW_LOW_RESOURCE_EXIT_REQUEST
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                    LOW_RESOURCE_MODE.store(false, std::sync::atomic::Ordering::SeqCst);
                    app_handle.exit(0);
                }
                _ => {}
            }
        }
        #[cfg(target_os = "windows")]
        tauri::RunEvent::TrayIconEvent(tray_event) => {
            if !LOW_RESOURCE_MODE.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }

            if let tauri::tray::TrayIconEvent::Click {
                id,
                button: tauri::tray::MouseButton::Left,
                ..
            } = tray_event
            {
                if id.as_ref() == "app-tray" {
                    tauri_plugin_eco_window::show_main_window(app_handle);
                }
            }
        }
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Reopen {
            has_visible_windows,
            ..
        } => {
            if has_visible_windows {
                return;
            }

            tauri_plugin_eco_window::show_preference_window(app_handle);
        }
        _ => {
            let _ = app_handle;
        }
    });
}
