// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use co_tauri::library::co_application::CoApplicationSettings;

#[tokio::main]
async fn main() {
	let co_settings = CoApplicationSettings::new("tauri-app").without_keychain();
	tauri::Builder::default()
		.plugin(tauri_plugin_shell::init())
		.plugin(co_tauri::init(co_settings).await)
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
