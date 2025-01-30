use tauri::{TitleBarStyle, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_co_sdk::library::co_application::CoApplicationSettings;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
	tauri::async_runtime::set(tokio::runtime::Handle::current());

	let co_settings = CoApplicationSettings::new("tauri-app").without_keychain();
	tauri::Builder::default()
		.setup(|app| {
			let win_builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
				.title("")
				.inner_size(1600.0, 1000.0);

			// set transparent title bar only when building for macOS
			#[cfg(target_os = "macos")]
			let win_builder = win_builder.title_bar_style(TitleBarStyle::Overlay);

			win_builder.focused(false).build()?;

			Ok(())
		})
		.plugin(tauri_plugin_co_sdk::init(co_settings).await)
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
