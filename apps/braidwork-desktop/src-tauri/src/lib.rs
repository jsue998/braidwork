//! Desktop adapter over canonical local projects.
//!
//! Services and IPC data are testable without a display. The `desktop` feature
//! enables the native Tauri host; Tauri development/build commands enable it.
#[cfg(feature = "desktop")]
mod commands;
mod creation;
pub mod dto;
pub mod error;
pub mod service;
mod state;

#[cfg(feature = "desktop")]
/// Runs the local native application with narrowly scoped plugins.
///
/// # Errors
/// Returns the native runtime failure if the window or plugins cannot initialize.
pub fn run() -> Result<(), tauri::Error> {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_opener::Builder::new()
                .open_js_links_on_click(false)
                .build(),
        )
        .manage(std::sync::Arc::new(service::DesktopService::default()))
        .invoke_handler(commands::handler())
        .run(tauri::generate_context!())
}
