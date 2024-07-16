use commands::{
	get_core_state::get_core_state,
	push::push,
	subscription::{subscribe, unsubscribe},
};
use library::{
	co_application::{application, CoApplicationSettings},
	subscription::Subscriptions,
};
use tauri::Wry;

pub mod commands;
pub mod library;

pub async fn tauri_builder(co_settings: CoApplicationSettings) -> tauri::Builder<Wry> {
	let application = application(co_settings).await;

	tauri::async_runtime::set(tokio::runtime::Handle::current());

	tauri::Builder::default()
		.plugin(tauri_plugin_shell::init())
		.manage(application)
		.manage(Subscriptions { active_subscriptions: Default::default() })
		.invoke_handler(tauri::generate_handler![get_core_state, push, subscribe, unsubscribe])
}
