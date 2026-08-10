//! Windows menu-mask key injection shared by global shortcuts.

use tauri_plugin_global_shortcut::Modifiers;
use winapi::um::winuser::{SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP};

/// `0xE8` is the unassigned virtual key used as a side-effect-free menu mask.
const VK_MENU_MASK: u16 = 0xE8;
const VK_CONTROL: u16 = 0x11;
const VK_SHIFT: u16 = 0x10;

/// Injects one unassigned key press while a system modifier is held.
///
/// Windows then treats the modifier as part of a chord instead of activating
/// the Start menu or an application's Alt menu when the modifier is released.
pub(super) fn suppress_menu_activation() {
    inject_key(VK_MENU_MASK);
}

/// Uses a modifier-only mask for Alt shortcuts so WebView2 does not enter
/// keyboard-navigation mode because of an unassigned virtual key event.
pub(super) fn suppress_alt_menu_activation(modifiers: Modifiers) {
    let virtual_key = if !modifiers.contains(Modifiers::CONTROL) {
        VK_CONTROL
    } else if !modifiers.contains(Modifiers::SHIFT) {
        VK_SHIFT
    } else {
        VK_MENU_MASK
    };

    inject_key(virtual_key);
}

fn inject_key(virtual_key: u16) {
    let mut inputs: [INPUT; 2] = unsafe { std::mem::zeroed() };

    inputs[0].type_ = INPUT_KEYBOARD;
    unsafe {
        *inputs[0].u.ki_mut() = KEYBDINPUT {
            wVk: virtual_key,
            wScan: 0,
            dwFlags: 0,
            time: 0,
            dwExtraInfo: 0,
        };
    }
    inputs[1].type_ = INPUT_KEYBOARD;
    unsafe {
        *inputs[1].u.ki_mut() = KEYBDINPUT {
            wVk: virtual_key,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };
    }

    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_mut_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    if sent as usize != inputs.len() {
        log::warn!("inject menu-mask key sent {sent}/{}", inputs.len());
    }
}
