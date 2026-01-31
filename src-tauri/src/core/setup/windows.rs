use rdev::{grab, Event, EventType, Key};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread::spawn,
};
use tauri::{AppHandle, Emitter, WebviewWindow};

static CTRL_PRESSED: AtomicBool = AtomicBool::new(false);
static SHIFT_PRESSED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DispatchEvent {
    code: &'static str,
    key_code: u32,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct InputEvent {
    key: String,
}

pub fn platform(
    app_handle: &AppHandle,
    main_window: WebviewWindow,
    _preference_window: WebviewWindow,
) {
    let app_handle = app_handle.clone();

    spawn(move || {
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
                        _ => {}
                    }

                    if let Ok(true) = main_window.is_visible() {
                        if handle_hotkey(&app_handle, key) {
                            return None;
                        }

                        // 只有窗口可见，且没有Ctrl修饰键被按下时，才派发输入事件给搜索框
                        let ctrl_pressed = CTRL_PRESSED.load(Ordering::Relaxed);

                        if !ctrl_pressed {
                            if let Some(input_key) = key_to_input_string(key) {
                                let _ = app_handle.emit("input-event", InputEvent {
                                    key: input_key,
                                });
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
                        _ => {}
                    }
                    Some(event)
                }

                _ => Some(event),
            }
        };

        if let Err(err) = grab(callback) {
            eprintln!("rdev grab error: {:?}", err);
        }
    });
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
        Return => Some(DispatchEvent {
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
        // 删除条目
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
        // 搜索框聚焦
        KeyF if ctrl_pressed => Some(DispatchEvent {
            code: "KeyF",
            key_code: 70,
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

fn key_to_input_string(key: Key) -> Option<String> {
    use Key::*;

    eprintln!("DEBUG key_to_input_string: {:?}", key);

    let shift_pressed = SHIFT_PRESSED.load(Ordering::Relaxed);

    match key {
        // 字母键
        KeyA => Some(if shift_pressed { "A" } else { "a" }.to_string()),
        KeyB => Some(if shift_pressed { "B" } else { "b" }.to_string()),
        KeyC => Some(if shift_pressed { "C" } else { "c" }.to_string()),
        KeyD => Some(if shift_pressed { "D" } else { "d" }.to_string()),
        KeyE => Some(if shift_pressed { "E" } else { "e" }.to_string()),
        KeyF => Some(if shift_pressed { "F" } else { "f" }.to_string()),
        KeyG => Some(if shift_pressed { "G" } else { "g" }.to_string()),
        KeyH => Some(if shift_pressed { "H" } else { "h" }.to_string()),
        KeyI => Some(if shift_pressed { "I" } else { "i" }.to_string()),
        KeyJ => Some(if shift_pressed { "J" } else { "j" }.to_string()),
        KeyK => Some(if shift_pressed { "K" } else { "k" }.to_string()),
        KeyL => Some(if shift_pressed { "L" } else { "l" }.to_string()),
        KeyM => Some(if shift_pressed { "M" } else { "m" }.to_string()),
        KeyN => Some(if shift_pressed { "N" } else { "n" }.to_string()),
        KeyO => Some(if shift_pressed { "O" } else { "o" }.to_string()),
        KeyP => Some(if shift_pressed { "P" } else { "p" }.to_string()),
        KeyQ => Some(if shift_pressed { "Q" } else { "q" }.to_string()),
        KeyR => Some(if shift_pressed { "R" } else { "r" }.to_string()),
        KeyS => Some(if shift_pressed { "S" } else { "s" }.to_string()),
        KeyT => Some(if shift_pressed { "T" } else { "t" }.to_string()),
        KeyU => Some(if shift_pressed { "U" } else { "u" }.to_string()),
        KeyV => Some(if shift_pressed { "V" } else { "v" }.to_string()),
        KeyW => Some(if shift_pressed { "W" } else { "w" }.to_string()),
        KeyX => Some(if shift_pressed { "X" } else { "x" }.to_string()),
        KeyY => Some(if shift_pressed { "Y" } else { "y" }.to_string()),
        KeyZ => Some(if shift_pressed { "Z" } else { "z" }.to_string()),
        // 数字键（小键盘）
        Kp0 => Some(if shift_pressed { ")" } else { "0" }.to_string()),
        Kp1 => Some(if shift_pressed { "!" } else { "1" }.to_string()),
        Kp2 => Some(if shift_pressed { "@" } else { "2" }.to_string()),
        Kp3 => Some(if shift_pressed { "#" } else { "3" }.to_string()),
        Kp4 => Some(if shift_pressed { "$" } else { "4" }.to_string()),
        Kp5 => Some(if shift_pressed { "%" } else { "5" }.to_string()),
        Kp6 => Some(if shift_pressed { "^" } else { "6" }.to_string()),
        Kp7 => Some(if shift_pressed { "&" } else { "7" }.to_string()),
        Kp8 => Some(if shift_pressed { "*" } else { "8" }.to_string()),
        Kp9 => Some(if shift_pressed { "(" } else { "9" }.to_string()),
        // 特殊键
        Space => Some(" ".to_string()),
        Backspace => Some("Backspace".to_string()),
        Delete => Some("Delete".to_string()),
        // 其他符号键
        Minus => Some(if shift_pressed { "_" } else { "-" }.to_string()),
        Equal => Some(if shift_pressed { "+" } else { "=" }.to_string()),
        LeftBracket => Some(if shift_pressed { "{" } else { "[" }.to_string()),
        RightBracket => Some(if shift_pressed { "}" } else { "]" }.to_string()),
        BackSlash => Some(if shift_pressed { "|" } else { "\\" }.to_string()),
        SemiColon => Some(if shift_pressed { ":" } else { ";" }.to_string()),
        Quote => Some(if shift_pressed { "\"" } else { "'" }.to_string()),
        Comma => Some(if shift_pressed { "<" } else { "," }.to_string()),
        Dot => Some(if shift_pressed { ">" } else { "." }.to_string()),
        Slash => Some(if shift_pressed { "?" } else { "/" }.to_string()),
        _ => None,
    }
}
