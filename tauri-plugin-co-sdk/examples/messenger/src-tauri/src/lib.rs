use tauri_plugin_co_sdk::library::co_application::CoApplicationSettings;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
	tauri::async_runtime::set(tokio::runtime::Handle::current());

	let co_settings = CoApplicationSettings::new("tauri-app").without_keychain();
	tauri::Builder::default()
		.plugin(tauri_plugin_co_sdk::init(co_settings).await)
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
