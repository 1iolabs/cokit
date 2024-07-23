use commands::{
	get_state::get_co_state,
	push::push,
	resolve_cid::resolve_cid,
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
		// TODO add feature that lets devs add their own commands as well? Maybe they need features
		// we didn't think about. Calling invoke_handler() again overwrites the already set handlers
		.invoke_handler(tauri::generate_handler![get_co_state, push, resolve_cid, subscribe, unsubscribe])
}
