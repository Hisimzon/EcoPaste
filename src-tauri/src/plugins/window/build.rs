const COMMANDS: &[&str] = &[
    "show_window",
    "hide_window",
    "set_low_resource_mode",
    "consume_low_resource_clipboard_dirty",
    "drain_low_resource_clipboard_queue",
    "show_taskbar_icon",
    "enter_search_mode",
    "exit_search_mode",
    "enter_input_mode",
    "exit_input_mode",
    "set_pinned",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
