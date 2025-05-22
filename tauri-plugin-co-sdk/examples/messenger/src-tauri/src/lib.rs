use tauri::{WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_co_sdk::library::co_application::CoApplicationSettings;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
	tauri::async_runtime::set(tokio::runtime::Handle::current());

	let co_settings = CoApplicationSettings::new("coapp-messenger-demo").without_keychain();
	tauri::Builder::default()
		.plugin(tauri_plugin_fs::init())
		.setup(|app| {
			let win_builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
				.title("CO Messenger Demo")
				.inner_size(1600.0, 1000.0);

			// set transparent title bar only when building for macOS
			#[cfg(target_os = "macos")]
			let win_builder = win_builder
				.title_bar_style(tauri::TitleBarStyle::Overlay)
				.traffic_light_position(tauri::LogicalPosition::new(22, 22))
				.hidden_title(true);

			win_builder.focused(false).always_on_bottom(false).position(0.0, 0.0).build()?;

			Ok(())
		})
		.plugin(tauri_plugin_co_sdk::init(co_settings).await)
		.plugin(tauri_plugin_dialog::init())
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
