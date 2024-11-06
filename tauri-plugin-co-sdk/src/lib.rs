use co_actor::Actor;
use commands::{
	get_state::get_co_state,
	push::push,
	resolve_cid::resolve_cid,
	storage::{storage_get, storage_set},
};
use futures::{pin_mut, StreamExt};
use library::{
	application_actor::{ApplicationActor, ApplicationActorMessage},
	co_application::CoApplicationSettings,
};
use tauri::{plugin::TauriPlugin, Emitter, Manager, Runtime};

pub mod commands;
pub mod library;

pub async fn init<R: Runtime>(co_settings: CoApplicationSettings) -> TauriPlugin<R> {
	// create an actor to handle application tasks

	// create a tauri plugin that acts as an api between frontends and co sdk
	tauri::plugin::Builder::new("co-sdk")
		.invoke_handler(tauri::generate_handler![get_co_state, push, resolve_cid, storage_get, storage_set])
		.setup(|app_handle, _api| {
			let actor_handle = Actor::spawn(Default::default(), ApplicationActor {}, co_settings)
				.unwrap()
				.handle();
			app_handle.manage(actor_handle.clone());
			tokio::spawn({
				let app_handle = app_handle.clone();
				async move {
					let stream = actor_handle.stream(ApplicationActorMessage::WatchState);
					pin_mut!(stream);
					while let Some(Ok(result)) = stream.next().await {
						app_handle.emit("co-sdk-new-state", result).ok();
					}
				}
			});
			Ok(())
		})
		.build()
}
