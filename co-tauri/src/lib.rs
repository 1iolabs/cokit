use commands::{get_state::get_co_state, push::push, resolve_cid::resolve_cid};
use library::co_application::{application, CoApplicationSettings};
use tauri::{plugin::TauriPlugin, Manager, Runtime};

pub mod commands;
pub mod library;

pub async fn init<R: Runtime>(co_settings: CoApplicationSettings) -> TauriPlugin<R> {
	let application = application(co_settings).await;
	tauri::plugin::Builder::new("tauri-plugin-co")
		.invoke_handler(tauri::generate_handler![get_co_state, push, resolve_cid])
		.setup(|app_handle, _api| {
			app_handle.manage(application);
			Ok(())
		})
		.build()
}
