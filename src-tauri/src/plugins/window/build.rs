const COMMANDS: &[&str] = &["show_window", "hide_window", "show_taskbar_icon", "enter_search_mode", "exit_search_mode"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
