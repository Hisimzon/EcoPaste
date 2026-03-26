use tauri_plugin_eco_window::{
    get_low_resource_clipboard_shortcut, show_main_window, INPUT_MODE, MAIN_WINDOW_LABEL,
    schedule_main_window_destroy, MAIN_WINDOW_TITLE, MAIN_WINDOW_VISIBLE,
    LOW_RESOURCE_DESTROY_DELAY_MS, LOW_RESOURCE_MODE, PINNED,
};
use rdev::{grab, listen, Button, Event, EventType, Key};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread::{sleep, spawn},
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use winapi::um::winuser::{GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW};

static CTRL_PRESSED: AtomicBool = AtomicBool::new(false);
static SHIFT_PRESSED: AtomicBool = AtomicBool::new(false);
static ALT_PRESSED: AtomicBool = AtomicBool::new(false);
static META_PRESSED: AtomicBool = AtomicBool::new(false);

fn reset_modifier_state() {
    CTRL_PRESSED.store(false, Ordering::Relaxed);
    SHIFT_PRESSED.store(false, Ordering::Relaxed);
    ALT_PRESSED.store(false, Ordering::Relaxed);
    META_PRESSED.store(false, Ordering::Relaxed);
}

fn is_foreground_main_window() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return false;
        }

        let length = GetWindowTextLengthW(hwnd);
        if length == 0 {
            return false;
        }

        let mut buffer: Vec<u16> = vec![0; (length + 1) as usize];
        GetWindowTextW(hwnd, buffer.as_mut_ptr(), length + 1);

        let title = String::from_utf16_lossy(&buffer[..length as usize]);
        title == MAIN_WINDOW_TITLE
    }
}

#[repr(C)]
struct POINT {
    x: i32,
    y: i32,
}

extern "system" {
    fn GetCursorPos(lp_point: *mut POINT) -> i32;
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DispatchEvent {
    code: &'static str,
    key_code: u32,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchInputEvent {
    char: String,
}

pub fn platform(
    app_handle: &AppHandle,
    main_window: WebviewWindow,
    _preference_window: WebviewWindow,
) {
    MAIN_WINDOW_VISIBLE.store(main_window.is_visible().unwrap_or(false), Ordering::Relaxed);

    let app_handle_for_keyboard = app_handle.clone();

    // 键盘事件监听（使用 grab 可以拦截按键）
    spawn(move || {
        loop {
            let app_handle_clone = app_handle_for_keyboard.clone();

            let callback = move |event: Event| -> Option<Event> {
                match event.event_type {
                    EventType::KeyPress(key) => {
                        match key {
                            Key::ControlLeft | Key::ControlRight => {
                                CTRL_PRESSED.store(true, Ordering::Relaxed)
                            }
                            Key::ShiftLeft | Key::ShiftRight => {
                                SHIFT_PRESSED.store(true, Ordering::Relaxed)
                            }
                            Key::Alt | Key::AltGr => {
                                ALT_PRESSED.store(true, Ordering::Relaxed)
                            }
                            Key::MetaLeft | Key::MetaRight => {
                                META_PRESSED.store(true, Ordering::Relaxed)
                            }
                            _ => {}
                        }

                        if LOW_RESOURCE_MODE.load(Ordering::Relaxed)
                            && !MAIN_WINDOW_VISIBLE.load(Ordering::Relaxed)
                        {
                            if handle_low_resource_wake_hotkey(key) {
                                // 低占用唤醒后重置修饰键，避免状态漂移导致搜索输入失效
                                reset_modifier_state();
                                show_main_window(&app_handle_clone);

                                return None;
                            }
                        }

                        if MAIN_WINDOW_VISIBLE.load(Ordering::Relaxed) {
                            // 输入模式时不拦截键盘，让用户可以正常输入（如备注 Modal）
                            if INPUT_MODE.load(Ordering::Relaxed) {
                                return Some(event);
                            }

                            if is_foreground_main_window()
                                && !LOW_RESOURCE_MODE.load(Ordering::Relaxed)
                            {
                                return Some(event);
                            }

                            // 拦截快捷键
                            if handle_hotkey(&app_handle_clone, key) {
                                return None;
                            }

                            // 捕获可打印字符用于搜索
                            // Ctrl 或 Alt 按下时不处理，避免组合键影响搜索框
                            let ctrl = CTRL_PRESSED.load(Ordering::Relaxed);
                            let alt = ALT_PRESSED.load(Ordering::Relaxed);
                            if !ctrl && !alt {
                                if let Some(ch) = key_to_char(key, SHIFT_PRESSED.load(Ordering::Relaxed)) {
                                    let _ = app_handle_clone.emit("search-input", SearchInputEvent { char: ch });
                                    // 拦截按键，避免字符输入到原窗口
                                    return None;
                                }
                            }
                        }

                        Some(event)
                    }

                    EventType::KeyRelease(key) => {
                        match key {
                            Key::ControlLeft | Key::ControlRight => {
                                CTRL_PRESSED.store(false, Ordering::Relaxed)
                            }
                            Key::ShiftLeft | Key::ShiftRight => {
                                SHIFT_PRESSED.store(false, Ordering::Relaxed)
                            }
                            Key::Alt | Key::AltGr => {
                                ALT_PRESSED.store(false, Ordering::Relaxed)
                            }
                            Key::MetaLeft | Key::MetaRight => {
                                META_PRESSED.store(false, Ordering::Relaxed)
                            }
                            _ => {}
                        }
                        Some(event)
                    }

                    _ => Some(event),
                }
            };

            if let Err(err) = grab(callback) {
                eprintln!("rdev grab error: {:?}", err);
                reset_modifier_state();

                sleep(Duration::from_millis(1200));

                continue;
            }

            break;
        }
    });

    // 鼠标事件监听（使用 listen 监听点击）
    let app_handle_clone = app_handle.clone();

    spawn(move || {
        let callback = move |event: Event| {
            if let EventType::ButtonPress(Button::Left) = event.event_type {
                if MAIN_WINDOW_VISIBLE.load(Ordering::Relaxed) {
                    let mut point = POINT { x: 0, y: 0 };
                    if unsafe { GetCursorPos(&mut point) } != 0 {
                        if let Some(window) = app_handle_clone.get_webview_window(MAIN_WINDOW_LABEL) {
                            if let (Ok(pos), Ok(size)) = (window.outer_position(), window.outer_size()) {
                            let win_x = pos.x;
                            let win_y = pos.y;
                            let win_r = win_x + size.width as i32;
                            let win_b = win_y + size.height as i32;

                            if point.x < win_x || point.x > win_r || point.y < win_y || point.y > win_b {
                                if !PINNED.load(Ordering::Relaxed) {
                                    MAIN_WINDOW_VISIBLE.store(false, Ordering::Relaxed);
                                    if LOW_RESOURCE_MODE.load(Ordering::Relaxed) {
                                        let _ = window.hide();
                                        schedule_main_window_destroy(
                                            &app_handle_clone,
                                            LOW_RESOURCE_DESTROY_DELAY_MS,
                                        );
                                    } else {
                                        let _ = window.hide();
                                    }
                                }
                            }
                            }
                        }
                    }
                }
            }
        };

        if let Err(err) = listen(callback) {
            eprintln!("rdev listen error: {:?}", err);
        }
    });
}

/// 将按键转换为字符（支持基本 ASCII 字符和数字小键盘）
fn key_to_char(key: Key, shift: bool) -> Option<String> {
    use Key::*;

    let ch = match key {
        // 字母键
        KeyA => if shift { 'A' } else { 'a' },
        KeyB => if shift { 'B' } else { 'b' },
        KeyC => if shift { 'C' } else { 'c' },
        KeyD => if shift { 'D' } else { 'd' },
        KeyE => if shift { 'E' } else { 'e' },
        KeyF => if shift { 'F' } else { 'f' },
        KeyG => if shift { 'G' } else { 'g' },
        KeyH => if shift { 'H' } else { 'h' },
        KeyI => if shift { 'I' } else { 'i' },
        KeyJ => if shift { 'J' } else { 'j' },
        KeyK => if shift { 'K' } else { 'k' },
        KeyL => if shift { 'L' } else { 'l' },
        KeyM => if shift { 'M' } else { 'm' },
        KeyN => if shift { 'N' } else { 'n' },
        KeyO => if shift { 'O' } else { 'o' },
        KeyP => if shift { 'P' } else { 'p' },
        KeyQ => if shift { 'Q' } else { 'q' },
        KeyR => if shift { 'R' } else { 'r' },
        KeyS => if shift { 'S' } else { 's' },
        KeyT => if shift { 'T' } else { 't' },
        KeyU => if shift { 'U' } else { 'u' },
        KeyV => if shift { 'V' } else { 'v' },
        KeyW => if shift { 'W' } else { 'w' },
        KeyX => if shift { 'X' } else { 'x' },
        KeyY => if shift { 'Y' } else { 'y' },
        KeyZ => if shift { 'Z' } else { 'z' },
        // 数字键
        Num0 => if shift { ')' } else { '0' },
        Num1 => if shift { '!' } else { '1' },
        Num2 => if shift { '@' } else { '2' },
        Num3 => if shift { '#' } else { '3' },
        Num4 => if shift { '$' } else { '4' },
        Num5 => if shift { '%' } else { '5' },
        Num6 => if shift { '^' } else { '6' },
        Num7 => if shift { '&' } else { '7' },
        Num8 => if shift { '*' } else { '8' },
        Num9 => if shift { '(' } else { '9' },
        // 数字小键盘
        Kp0 => '0',
        Kp1 => '1',
        Kp2 => '2',
        Kp3 => '3',
        Kp4 => '4',
        Kp5 => '5',
        Kp6 => '6',
        Kp7 => '7',
        Kp8 => '8',
        Kp9 => '9',
        KpMinus => '-',
        KpPlus => '+',
        KpMultiply => '*',
        KpDivide => '/',
        // 符号键
        Minus => if shift { '_' } else { '-' },
        Equal => if shift { '+' } else { '=' },
        LeftBracket => if shift { '{' } else { '[' },
        RightBracket => if shift { '}' } else { ']' },
        BackSlash => if shift { '|' } else { '\\' },
        SemiColon => if shift { ':' } else { ';' },
        Quote => if shift { '"' } else { '\'' },
        Comma => if shift { '<' } else { ',' },
        Dot => if shift { '>' } else { '.' },
        Slash => if shift { '?' } else { '/' },
        BackQuote => if shift { '~' } else { '`' },
        // Space 已被 handle_hotkey 拦截用于预览，不用于搜索输入
        _ => return None,
    };

    Some(ch.to_string())
}

fn handle_low_resource_wake_hotkey(key: Key) -> bool {
    let shortcut = get_low_resource_clipboard_shortcut();

    is_shortcut_match(&shortcut, key)
}

fn is_shortcut_match(shortcut: &str, key: Key) -> bool {
    let Some((need_ctrl, need_shift, need_alt, need_meta, target_key)) =
        parse_shortcut(shortcut)
    else {
        return false;
    };

    let current_ctrl = CTRL_PRESSED.load(Ordering::Relaxed);
    let current_shift = SHIFT_PRESSED.load(Ordering::Relaxed);
    let current_alt = ALT_PRESSED.load(Ordering::Relaxed);
    let current_meta = META_PRESSED.load(Ordering::Relaxed);

    (!need_ctrl || current_ctrl)
        && (!need_shift || current_shift)
        && (!need_alt || current_alt)
        && (!need_meta || current_meta)
        && key == target_key
}

fn parse_shortcut(shortcut: &str) -> Option<(bool, bool, bool, bool, Key)> {
    let mut need_ctrl = false;
    let mut need_shift = false;
    let mut need_alt = false;
    let mut need_meta = false;
    let mut key_token: Option<String> = None;

    for item in shortcut
        .split('+')
        .map(|item| item.trim().to_ascii_lowercase())
        .filter(|item| !item.is_empty())
    {
        match item.as_str() {
            "ctrl" | "control" | "commandorcontrol" | "controlorcommand" => {
                need_ctrl = true;
            }
            "shift" => {
                need_shift = true;
            }
            "alt" | "option" => {
                need_alt = true;
            }
            "meta" | "cmd" | "command" | "super" | "win" | "windows" => {
                need_meta = true;
            }
            _ => {
                key_token = Some(item);
            }
        }
    }

    let target_key = parse_key_token(&key_token?)?;

    Some((need_ctrl, need_shift, need_alt, need_meta, target_key))
}

fn parse_key_token(token: &str) -> Option<Key> {
    use Key::*;

    let key = match token {
        "a" => KeyA,
        "b" => KeyB,
        "c" => KeyC,
        "d" => KeyD,
        "e" => KeyE,
        "f" => KeyF,
        "g" => KeyG,
        "h" => KeyH,
        "i" => KeyI,
        "j" => KeyJ,
        "k" => KeyK,
        "l" => KeyL,
        "m" => KeyM,
        "n" => KeyN,
        "o" => KeyO,
        "p" => KeyP,
        "q" => KeyQ,
        "r" => KeyR,
        "s" => KeyS,
        "t" => KeyT,
        "u" => KeyU,
        "v" => KeyV,
        "w" => KeyW,
        "x" => KeyX,
        "y" => KeyY,
        "z" => KeyZ,
        "0" => Num0,
        "1" => Num1,
        "2" => Num2,
        "3" => Num3,
        "4" => Num4,
        "5" => Num5,
        "6" => Num6,
        "7" => Num7,
        "8" => Num8,
        "9" => Num9,
        "space" => Space,
        "tab" => Tab,
        "enter" | "return" => Return,
        "escape" | "esc" => Escape,
        "backspace" => Backspace,
        "delete" | "del" => Delete,
        "comma" | "," => Comma,
        "period" | "dot" | "." => Dot,
        "minus" | "-" => Minus,
        "equal" | "equals" | "=" => Equal,
        _ => return None,
    };

    Some(key)
}

fn handle_hotkey(app_handle: &AppHandle, key: Key) -> bool {
    use Key::*;

    let ctrl_pressed = CTRL_PRESSED.load(Ordering::Relaxed);
    let shift_pressed = SHIFT_PRESSED.load(Ordering::Relaxed);

    let event = match key {
        // 预览
        Space => Some(DispatchEvent {
            code: "Space",
            key_code: 32,
        }),
        // 选择上一个
        UpArrow => Some(DispatchEvent {
            code: "ArrowUp",
            key_code: 38,
        }),
        // 选择下一个
        DownArrow => Some(DispatchEvent {
            code: "ArrowDown",
            key_code: 40,
        }),
        // 粘贴
        Return | KpReturn => Some(DispatchEvent {
            code: "Enter",
            key_code: 13,
        }),
        // 选择上一个分组
        Tab if shift_pressed => Some(DispatchEvent {
            code: "Tab",
            key_code: 9,
        }),
        // 选择下一个分组
        Tab => Some(DispatchEvent {
            code: "Tab",
            key_code: 9,
        }),
        // 滚动到顶部
        Home => Some(DispatchEvent {
            code: "Home",
            key_code: 36,
        }),
        // 清空搜索框
        Escape => Some(DispatchEvent {
            code: "Escape",
            key_code: 27,
        }),
        // 删除条目或搜索框字符
        Backspace => Some(DispatchEvent {
            code: "Backspace",
            key_code: 8,
        }),
        Delete => Some(DispatchEvent {
            code: "Delete",
            key_code: 46,
        }),
        // 收藏条目
        KeyD if ctrl_pressed => Some(DispatchEvent {
            code: "KeyD",
            key_code: 68,
        }),
        // 固定窗口
        KeyP if ctrl_pressed => Some(DispatchEvent {
            code: "KeyP",
            key_code: 80,
        }),
        // 打开偏好设置
        Comma if ctrl_pressed => Some(DispatchEvent {
            code: "Comma",
            key_code: 188,
        }),
        // 隐藏窗口
        KeyW if ctrl_pressed => Some(DispatchEvent {
            code: "KeyW",
            key_code: 87,
        }),
        _ => None,
    };

    if let Some(ev) = event {
        let _ = app_handle.emit("dispatch-event", ev);

        return true;
    }

    false
}
