//! 列表项右键菜单（Rust 侧）：macOS 走原生 muda；Windows 生成菜单 payload，
//! 由剪贴板主 WebView 渲染，避免 `TrackPopupMenu` 抢前台焦点。

pub mod clipboard_item;
