//! Windows 专属：用低级键盘钩子接管 `Win+V`。
//!
//! `Win+V` 是 Windows 系统保留热键，`RegisterHotKey(MOD_WIN, V)`（全局快捷键插件底层）
//! 无法拦截、也无法阻止系统剪贴板历史面板弹出。因此这里装一颗 `WH_KEYBOARD_LL` 钩子，
//! 在 V 按下时若检测到 Win 键按住就吞掉该按键并 toggle 剪贴板窗口，使系统面板不再触发。
//!
//! 与 `keyboard/`（剪贴板窗口可见期间捕获导航键）不同：本钩子的生命周期由设置开关驱动，
//! 开启即常驻，与剪贴板窗口可见性无关。

use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use tauri::AppHandle;
use winapi::shared::minwindef::{DWORD, LPARAM, LRESULT, UINT, ULONG, WPARAM};
use winapi::shared::windef::HHOOK;
use winapi::shared::winerror::ERROR_INVALID_HOOK_HANDLE;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::powerbase::{
    PowerRegisterSuspendResumeNotification, PowerUnregisterSuspendResumeNotification,
};
use winapi::um::powrprof::{
    DEVICE_NOTIFY_CALLBACK, DEVICE_NOTIFY_CALLBACK_ROUTINE, DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS,
};
use winapi::um::processthreadsapi::GetCurrentThreadId;
use winapi::um::winnt::PVOID;
use winapi::um::winuser::{
    CallNextHookEx, GetAsyncKeyState, GetMessageW, PeekMessageW, PostThreadMessageW,
    SetWindowsHookExW, UnhookWindowsHookEx, HPOWERNOTIFY, KBDLLHOOKSTRUCT, LLKHF_EXTENDED,
    LLKHF_INJECTED, MSG, PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMESUSPEND, PM_NOREMOVE, VK_CONTROL,
    VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN,
    VK_SHIFT, WH_KEYBOARD_LL, WM_APP, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use super::mask_key;
use crate::window::{self, CLIPBOARD_WINDOW_LABEL};

/// V 键的虚拟键码，winapi 未直接导出。
const VK_V: u32 = 0x56;
const WM_APPLY_ENABLED: UINT = WM_APP + 1;
const WM_RESUME_DETECTED: UINT = WM_APP + 2;
const WM_REINSTALL_HOOK: UINT = WM_APP + 3;
const RESUME_REINSTALL_DELAY: Duration = Duration::from_millis(750);
const MOD_LWIN: u32 = 1 << 0;
const MOD_RWIN: u32 = 1 << 1;
const MOD_LCONTROL: u32 = 1 << 2;
const MOD_RCONTROL: u32 = 1 << 3;
const MOD_LSHIFT: u32 = 1 << 4;
const MOD_RSHIFT: u32 = 1 << 5;
const MOD_LALT: u32 = 1 << 6;
const MOD_RALT: u32 = 1 << 7;
const MOD_WIN: u32 = MOD_LWIN | MOD_RWIN;
const MOD_EXTRA: u32 = MOD_LCONTROL | MOD_RCONTROL | MOD_LSHIFT | MOD_RSHIFT | MOD_LALT | MOD_RALT;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ControllerPhase {
    Stopped,
    Starting,
    Running,
}

#[derive(Debug)]
struct ControllerState {
    phase: ControllerPhase,
    thread_id: Option<u32>,
    generation: u32,
}

impl ControllerState {
    const fn new() -> Self {
        Self {
            phase: ControllerPhase::Stopped,
            thread_id: None,
            generation: 0,
        }
    }
}

static DESIRED_ENABLED: AtomicBool = AtomicBool::new(false);
static V_DOWN: AtomicBool = AtomicBool::new(false);
static V_CONSUMED: AtomicBool = AtomicBool::new(false);
static MODIFIER_STATE: AtomicU32 = AtomicU32::new(0);
static RESUME_EPOCH: AtomicU32 = AtomicU32::new(0);
static REINSTALL_PENDING: AtomicBool = AtomicBool::new(false);
static CONTROLLER_THREAD_ID: AtomicU32 = AtomicU32::new(0);
static CONTROLLER: Mutex<ControllerState> = Mutex::new(ControllerState::new());
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

/// 按设置开关启停 Win+V 接管钩子；幂等，可重复调用。
pub fn set_enabled(app: &AppHandle, enabled: bool) {
    let _ = APP_HANDLE.set(app.clone());
    DESIRED_ENABLED.store(enabled, Ordering::Release);

    let thread_id = {
        let controller = CONTROLLER.lock().expect("win_v controller state poisoned");
        controller.thread_id
    };

    if let Some(thread_id) = thread_id {
        post_controller_message(thread_id, WM_APPLY_ENABLED, 0);

        return;
    }

    if !enabled {
        return;
    }

    let generation = {
        let mut controller = CONTROLLER.lock().expect("win_v controller state poisoned");
        if controller.phase != ControllerPhase::Stopped {
            return;
        }

        controller.generation = controller.generation.wrapping_add(1);
        controller.phase = ControllerPhase::Starting;
        controller.generation
    };

    if let Err(err) = std::thread::Builder::new()
        .name("win-v-hook".into())
        .spawn(move || run_controller(generation))
    {
        let mut controller = CONTROLLER.lock().expect("win_v controller state poisoned");
        if controller.phase == ControllerPhase::Starting
            && controller.generation == generation
        {
            controller.phase = ControllerPhase::Stopped;
        }
        log::error!("spawn win+v controller thread failed: {err}");
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }

    let kbd = &*(lparam as *const KBDLLHOOKSTRUCT);

    // 保持既有策略：所有注入事件完全放行，避免自动化工具造成状态残留或触发回环。
    if kbd.flags & LLKHF_INJECTED != 0 {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }

    let vk = kbd.vkCode;
    let msg = wparam as UINT;
    update_modifier_state(vk, kbd.scanCode, kbd.flags, msg);

    if vk != VK_V {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }

    if !DESIRED_ENABLED.load(Ordering::Acquire) {
        if (msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN)
            && V_CONSUMED.load(Ordering::Relaxed)
        {
            return 1;
        }

        if (msg == WM_KEYUP || msg == WM_SYSKEYUP) && V_CONSUMED.swap(false, Ordering::Relaxed) {
            V_DOWN.store(false, Ordering::Relaxed);
            let thread_id = CONTROLLER_THREAD_ID.load(Ordering::Acquire);
            if thread_id != 0 {
                PostThreadMessageW(thread_id, WM_APPLY_ENABLED, 0, 0);
            }

            return 1;
        }

        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }

    if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
        if V_DOWN.swap(true, Ordering::Relaxed) {
            if V_CONSUMED.load(Ordering::Relaxed) {
                return 1;
            }

            return CallNextHookEx(null_mut(), code, wparam, lparam);
        }

        if !is_pure_win(MODIFIER_STATE.load(Ordering::Relaxed)) {
            return CallNextHookEx(null_mut(), code, wparam, lparam);
        }

        V_CONSUMED.store(true, Ordering::Relaxed);
        mask_key::suppress_menu_activation();
        schedule_toggle();

        return 1;
    }

    if msg == WM_KEYUP || msg == WM_SYSKEYUP {
        V_DOWN.store(false, Ordering::Relaxed);
        if V_CONSUMED.swap(false, Ordering::Relaxed) {
            if REINSTALL_PENDING.swap(false, Ordering::AcqRel) {
                let thread_id = CONTROLLER_THREAD_ID.load(Ordering::Acquire);
                if thread_id != 0 {
                    PostThreadMessageW(
                        thread_id,
                        WM_REINSTALL_HOOK,
                        RESUME_EPOCH.load(Ordering::Acquire) as WPARAM,
                        0,
                    );
                }
            }

            // 配对吞掉 V 抬起：按下已被拦截，放行抬起会让前台应用收到孤立 KEYUP。
            return 1;
        }
    }

    CallNextHookEx(null_mut(), code, wparam, lparam)
}

/// 控制线程串行安装、卸载与恢复 hook，避免快速开关产生重叠实例。
fn run_controller(generation: u32) {
    unsafe {
        let thread_id = GetCurrentThreadId();
        let mut msg: MSG = std::mem::zeroed();
        PeekMessageW(&mut msg, null_mut(), 0, 0, PM_NOREMOVE);

        {
            let mut controller = CONTROLLER.lock().expect("win_v controller state poisoned");
            if controller.phase != ControllerPhase::Starting || controller.generation != generation
            {
                return;
            }

            controller.phase = ControllerPhase::Running;
            controller.thread_id = Some(thread_id);
            CONTROLLER_THREAD_ID.store(thread_id, Ordering::Release);
        }

        let mut hook: HHOOK = null_mut();
        let mut power_callback: DEVICE_NOTIFY_CALLBACK_ROUTINE =
            Some(power_notification_callback);
        let mut power_parameters = DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS {
            Callback: &mut power_callback,
            Context: null_mut(),
        };
        let power_notification = register_power_notification(&mut power_parameters);
        apply_desired_hook_state(&mut hook);

        loop {
            let result = GetMessageW(&mut msg, null_mut(), 0, 0);
            if result <= 0 {
                break;
            }

            match msg.message {
                WM_APPLY_ENABLED => apply_desired_hook_state(&mut hook),
                WM_RESUME_DETECTED => schedule_resume_reinstall(generation, thread_id),
                WM_REINSTALL_HOOK => {
                    let epoch = msg.wParam as u32;
                    if epoch == RESUME_EPOCH.load(Ordering::Acquire)
                        && DESIRED_ENABLED.load(Ordering::Acquire)
                    {
                        if V_CONSUMED.load(Ordering::Relaxed) && is_key_down(VK_V as i32) {
                            REINSTALL_PENDING.store(true, Ordering::Release);
                        } else if uninstall_hook(&mut hook) {
                            REINSTALL_PENDING.store(false, Ordering::Release);
                            install_hook(&mut hook);
                        }
                    }
                }
                _ => {}
            }
        }

        let _ = uninstall_hook(&mut hook);
        if !power_notification.is_null() {
            let result = PowerUnregisterSuspendResumeNotification(power_notification);
            if result != 0 {
                log::warn!("unregister win+v suspend/resume notification failed: {result}");
            }
        }

        let mut controller = CONTROLLER.lock().expect("win_v controller state poisoned");
        if controller.generation == generation {
            controller.phase = ControllerPhase::Stopped;
            controller.thread_id = None;
            CONTROLLER_THREAD_ID.store(0, Ordering::Release);
        }
    }
}

/// 根据最终设置状态在控制线程内安装或卸载 hook。
unsafe fn apply_desired_hook_state(hook: &mut HHOOK) {
    if DESIRED_ENABLED.load(Ordering::Acquire) {
        install_hook(hook);
    } else if !V_CONSUMED.load(Ordering::Relaxed) {
        let _ = uninstall_hook(hook);
    }
}

/// 安装唯一的 Win+V hook；已有实例时保持幂等。
unsafe fn install_hook(hook: &mut HHOOK) {
    if !hook.is_null() {
        return;
    }

    let installed = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), null_mut(), 0);
    if installed.is_null() {
        log::error!("SetWindowsHookExW(WH_KEYBOARD_LL) for win+v failed");

        return;
    }

    *hook = installed;
    synchronize_key_state();
}

/// 卸载当前 Win+V hook，并清理未完成的 V 按键配对状态。
unsafe fn uninstall_hook(hook: &mut HHOOK) -> bool {
    if hook.is_null() {
        reset_v_state();

        return true;
    }

    if UnhookWindowsHookEx(*hook) == 0 {
        let error = GetLastError();
        if error != ERROR_INVALID_HOOK_HANDLE {
            log::warn!("UnhookWindowsHookEx for win+v failed: {error}");

            return false;
        }
    }

    *hook = null_mut();
    reset_v_state();

    true
}

/// 注册系统 suspend/resume callback；失败仅降级为不自动恢复，不影响 Win+V 本身。
unsafe fn register_power_notification(
    parameters: &mut DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS,
) -> HPOWERNOTIFY {
    let mut registration: HPOWERNOTIFY = null_mut();
    let result = PowerRegisterSuspendResumeNotification(
        DEVICE_NOTIFY_CALLBACK,
        parameters as *mut DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS as PVOID,
        &mut registration,
    );
    if result != 0 {
        log::warn!("register win+v suspend/resume notification failed: {result}");

        return null_mut();
    }

    registration
}

/// 电源回调只投递线程消息，hook 重装仍由原控制线程串行执行。
unsafe extern "system" fn power_notification_callback(
    _context: PVOID,
    event_type: ULONG,
    _setting: PVOID,
) -> ULONG {
    if event_type as WPARAM == PBT_APMRESUMEAUTOMATIC
        || event_type as WPARAM == PBT_APMRESUMESUSPEND
    {
        if !DESIRED_ENABLED.load(Ordering::Acquire) {
            return 0;
        }

        let thread_id = CONTROLLER_THREAD_ID.load(Ordering::Acquire);
        if thread_id != 0 {
            PostThreadMessageW(thread_id, WM_RESUME_DETECTED, 0, 0);
        }
    }

    0
}

/// 合并重复唤醒通知，延迟到 Windows 输入服务稳定后再请求重装。
fn schedule_resume_reinstall(generation: u32, thread_id: u32) {
    let epoch = RESUME_EPOCH.fetch_add(1, Ordering::AcqRel).wrapping_add(1);

    std::thread::spawn(move || {
        std::thread::sleep(RESUME_REINSTALL_DELAY);

        let is_current = {
            let controller = CONTROLLER.lock().expect("win_v controller state poisoned");
            controller.phase == ControllerPhase::Running
                && controller.generation == generation
                && controller.thread_id == Some(thread_id)
        };
        if !is_current || epoch != RESUME_EPOCH.load(Ordering::Acquire) {
            return;
        }

        post_controller_message(thread_id, WM_REINSTALL_HOOK, epoch as WPARAM);
    });
}

fn post_controller_message(thread_id: u32, message: UINT, wparam: WPARAM) {
    unsafe {
        if PostThreadMessageW(thread_id, message, wparam, 0) == 0 {
            log::warn!("post win+v controller message {message} failed");
        }
    }
}

fn reset_v_state() {
    V_DOWN.store(false, Ordering::Relaxed);
    V_CONSUMED.store(false, Ordering::Relaxed);
    REINSTALL_PENDING.store(false, Ordering::Relaxed);
}

fn is_pure_win(modifiers: u32) -> bool {
    modifiers & MOD_WIN != 0 && modifiers & MOD_EXTRA == 0
}

/// 安装或恢复 hook 时同步真实按键状态，避免从按键中途开始误判为新组合。
unsafe fn synchronize_key_state() {
    MODIFIER_STATE.store(read_modifier_state(), Ordering::Relaxed);
    V_DOWN.store(is_key_down(VK_V as i32), Ordering::Relaxed);
    V_CONSUMED.store(false, Ordering::Relaxed);
}

/// 用低级键盘事件维护左右修饰键，避免异步状态查询误判组合键。
fn update_modifier_state(vk: DWORD, scan_code: DWORD, flags: DWORD, message: UINT) {
    let Some(mask) = modifier_mask(vk, scan_code, flags) else {
        return;
    };

    if message == WM_KEYDOWN || message == WM_SYSKEYDOWN {
        MODIFIER_STATE.fetch_or(mask, Ordering::Relaxed);
    } else if message == WM_KEYUP || message == WM_SYSKEYUP {
        MODIFIER_STATE.fetch_and(!mask, Ordering::Relaxed);
    }
}

/// 将通用或左右修饰键的 Win32 键码归一化为内部状态位。
fn modifier_mask(vk: DWORD, scan_code: DWORD, flags: DWORD) -> Option<u32> {
    match vk as i32 {
        VK_LWIN => Some(MOD_LWIN),
        VK_RWIN => Some(MOD_RWIN),
        VK_LCONTROL => Some(MOD_LCONTROL),
        VK_RCONTROL => Some(MOD_RCONTROL),
        VK_LSHIFT => Some(MOD_LSHIFT),
        VK_RSHIFT => Some(MOD_RSHIFT),
        VK_LMENU => Some(MOD_LALT),
        VK_RMENU => Some(MOD_RALT),
        VK_CONTROL => Some(if flags & LLKHF_EXTENDED != 0 {
            MOD_RCONTROL
        } else {
            MOD_LCONTROL
        }),
        VK_SHIFT => Some(if scan_code == 0x36 {
            MOD_RSHIFT
        } else {
            MOD_LSHIFT
        }),
        VK_MENU => Some(if flags & LLKHF_EXTENDED != 0 {
            MOD_RALT
        } else {
            MOD_LALT
        }),
        _ => None,
    }
}

/// hook 安装时读取当前真实修饰键状态，补齐安装前已经按下的按键。
unsafe fn read_modifier_state() -> u32 {
    let mut state = 0;
    for (key, mask) in [
        (VK_LWIN, MOD_LWIN),
        (VK_RWIN, MOD_RWIN),
        (VK_LCONTROL, MOD_LCONTROL),
        (VK_RCONTROL, MOD_RCONTROL),
        (VK_LSHIFT, MOD_LSHIFT),
        (VK_RSHIFT, MOD_RSHIFT),
        (VK_LMENU, MOD_LALT),
        (VK_RMENU, MOD_RALT),
    ] {
        if is_key_down(key) {
            state |= mask;
        }
    }

    state
}

unsafe fn is_key_down(key: i32) -> bool {
    (GetAsyncKeyState(key) as u16) & 0x8000 != 0
}

#[cfg(test)]
mod tests {
    use super::{
        is_pure_win, MOD_LALT, MOD_LCONTROL, MOD_LSHIFT, MOD_LWIN, MOD_RALT, MOD_RCONTROL,
        MOD_RSHIFT, MOD_RWIN,
    };

    #[test]
    fn missing_win_modifier_excludes_takeover() {
        assert!(!is_pure_win(0));
        assert!(!is_pure_win(MOD_LSHIFT));
    }

    #[test]
    fn pure_win_modifier_state_has_no_extra_modifiers() {
        for modifiers in [MOD_LWIN, MOD_RWIN, MOD_LWIN | MOD_RWIN] {
            assert!(is_pure_win(modifiers));
        }
    }

    #[test]
    fn additional_modifiers_exclude_win_v_takeover() {
        for modifier in [
            MOD_LSHIFT,
            MOD_RSHIFT,
            MOD_LCONTROL,
            MOD_RCONTROL,
            MOD_LALT,
            MOD_RALT,
        ] {
            let modifiers = MOD_LWIN | modifier;

            assert!(!is_pure_win(modifiers));
        }
    }
}

/// 钩子线程不能直接操作窗口，回到主线程 toggle 剪贴板窗口。
fn schedule_toggle() {
    let Some(app) = APP_HANDLE.get() else {
        return;
    };

    let handle = app.clone();
    if let Err(err) = app.run_on_main_thread(move || {
        if let Err(err) = window::toggle_window(&handle, CLIPBOARD_WINDOW_LABEL) {
            log::warn!("toggle clipboard window via win+v failed: {err}");
        }
    }) {
        log::warn!("schedule win+v toggle failed: {err}");
    }
}
