// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use co_tauri::library::co_settings::CoSettings;

#[tokio::main]
async fn main() {
	let co_settings = CoSettings::new("tauri-app");
	co_tauri::tauri_builder(co_settings)
		.await
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
