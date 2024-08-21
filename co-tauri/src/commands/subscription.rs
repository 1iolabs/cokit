use crate::library::{
	subscription::{build_event_name, Subscriptions},
	tauri_error::CoTauriError,
};
use anyhow::anyhow;
use co_log::SignedEntry;
use co_primitives::ReducerAction;
use co_sdk::{Application, BlockStorageExt, CoId};
use futures::StreamExt;
use libipld::Ipld;
use std::collections::BTreeSet;
use tauri::Emitter;
use tokio_stream::wrappers::WatchStream;

#[tauri::command]
pub async fn subscribe(
	application: tauri::State<'_, Application>,
	subscriptions: tauri::State<'_, Subscriptions>,
	app: tauri::AppHandle,
	co: CoId,
	core: Option<&str>,
	source: &str,
) -> Result<(), CoTauriError> {
	tracing::info!("tauri command subscribe: {:#?}", co);
	let co_reducer = application
		.co_reducer(co.clone())
		.await?
		.ok_or(anyhow!("Co not found: {}", co))?;

	let event = build_event_name(co, core);

	// scope to ensure lock gets released
	{
		let mut subscriptions = subscriptions.active_subscriptions.lock().await;
		if let Some(subscribers) = subscriptions.get_mut(&event) {
			// event is already being emitted by another watcher
			// -> add as subscriber and return (prevents multiple event emissions)
			subscribers.insert(source.to_owned());
			return Ok(());
		} else {
			// new event -> create subscribers set and update
			let mut map: BTreeSet<String> = Default::default();
			map.insert(source.to_owned());
			subscriptions.insert(event.to_owned(), map);
		}
	}
	let mut watcher = WatchStream::from_changes(co_reducer.watch().await);
	while let Some(item) = watcher.next().await {
		match item {
			Some((_, heads)) => {
				let head: SignedEntry = co_reducer.storage().get_deserialized(heads.first().unwrap()).await.unwrap();
				let payload: ReducerAction<Ipld> =
					co_reducer.storage().get_deserialized(&head.entry.payload).await.unwrap();
				// filter actions from other cores if a core is given
				if let Some(core) = core {
					if payload.core != core {
						continue;
					}
				}
				app.emit(&event, payload).unwrap();
			},
			None => (),
		};
		// stop emitting events if no subs left
		if subscriptions.active_subscriptions.lock().await.get(&event).is_none() {
			tracing::info!("tauri watch end: {event}");
			return Ok(());
		}
	}
	Ok(())
}

#[tauri::command]
pub async fn unsubscribe(
	subscriptions: tauri::State<'_, Subscriptions>,
	co: CoId,
	core: Option<&str>,
	source: &str,
) -> Result<(), CoTauriError> {
	let event = build_event_name(co, core);
	let mut subscriptions = subscriptions.active_subscriptions.lock().await;
	// remove subscribers for event type
	if let Some(mut subscribers) = subscriptions.remove(&event) {
		subscribers.remove(source);
		// add list back if other subscribers are remaining
		if subscribers.len() > 0 {
			subscriptions.insert(event, subscribers);
		}
	}
	Ok(())
}
