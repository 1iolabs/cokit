use anyhow::anyhow;
use co_log::SignedEntry;
use co_primitives::ReducerAction;
use co_sdk::{Application, ApplicationBuilder, BlockStorageExt, CoId};
use libipld::{cbor::DagCborCodec, codec::Codec, Cid, Ipld};
use library::{co_settings::CoSettings, subscription::build_event_name};
use serde::Deserialize;
use std::{
	collections::{BTreeMap, BTreeSet},
	sync::Mutex,
};
use tauri::{ipc::InvokeError, Manager, Wry};
use tokio_stream::{wrappers::WatchStream, StreamExt};
pub mod library;

async fn application(settings: CoSettings) -> Application {
	let identifier = settings.identifier;
	let builder = match settings.path {
		Some(path) => ApplicationBuilder::new_with_path(identifier, path),
		None => ApplicationBuilder::new(identifier),
	};
	let mut application = builder
		.without_keychain()
		.with_bunyan_logging(None)
		.build()
		.await
		.expect("application");

	// network
	if settings.network {
		application
			.create_network(settings.network_force_new_peer_id)
			.await
			.expect("network");
	}
	application.clone()
}

struct Subscriptions {
	active_subscriptions: Mutex<BTreeMap<String, BTreeSet<String>>>,
}

#[derive(Debug)]
struct CoTauriError {
	error: anyhow::Error,
}

impl From<CoTauriError> for InvokeError {
	fn from(val: CoTauriError) -> Self {
		InvokeError::from_anyhow(val.error)
	}
}

impl From<anyhow::Error> for CoTauriError {
	fn from(error: anyhow::Error) -> Self {
		Self { error }
	}
}

#[tauri::command]
async fn get_core_state(
	application: tauri::State<'_, Application>,
	co: CoId,
	_core: String,
) -> Result<(Option<Cid>, BTreeSet<Cid>), CoTauriError> {
	let reducer = application
		.co_reducer(co.clone())
		.await?
		.ok_or(anyhow!("Co not found: {}", co.clone()))?;
	let (state, heads) = reducer.reducer_state().await;
	// TODO resolve state up to declared core
	Ok((state, heads))
}

#[derive(Deserialize, Debug)]
struct PushCommandBody {
	co: CoId,
	core: String,
	action: Ipld,
}

impl TryFrom<Ipld> for PushCommandBody {
	type Error = anyhow::Error;
	fn try_from(value: Ipld) -> Result<Self, Self::Error> {
		match value {
			Ipld::Map(map) => {
				let action = PushCommandBody::resolve_action(&map)?;
				let co = PushCommandBody::resolve_co_id(&map)?;
				let core = PushCommandBody::resolve_core(&map)?;
				Ok(PushCommandBody { action, co, core })
			},
			_ => Err(anyhow!("Ipld is not a map")),
		}
	}
}

impl PushCommandBody {
	fn resolve_action(map: &BTreeMap<String, Ipld>) -> Result<Ipld, anyhow::Error> {
		if let Some(action) = map.get("action") {
			Ok(action.clone())
		} else {
			Err(anyhow!("Body contains no action"))
		}
	}
	fn resolve_co_id(map: &BTreeMap<String, Ipld>) -> Result<CoId, anyhow::Error> {
		if let Some(ipld) = map.get("co") {
			match ipld {
				Ipld::String(co) => Ok(CoId::new(&*co)),
				_ => Err(anyhow!("Co is not a string")),
			}
		} else {
			Err(anyhow!("Body contains no co info"))
		}
	}
	fn resolve_core(map: &BTreeMap<String, Ipld>) -> Result<String, anyhow::Error> {
		if let Some(ipld) = map.get("core") {
			match ipld {
				Ipld::String(core) => Ok(core.clone()),
				_ => Err(anyhow!("core not a string")),
			}
		} else {
			Err(anyhow!("body contains no core info"))
		}
	}
}

#[tauri::command]
async fn push(application: tauri::State<'_, Application>, body: Vec<u8>) -> Result<(), CoTauriError> {
	let body: PushCommandBody = DagCborCodec::default().decode::<Ipld>(&body)?.try_into()?;
	tracing::info!("tauri command push: {:#?}", body);
	let reducer = application
		.co_reducer(body.co.clone())
		.await?
		.ok_or(anyhow!("Co not found: {}", body.co))?;
	let identity = application.local_identity();
	reducer.push(&identity, &body.core, &body.action).await?;
	Ok(())
}

#[tauri::command]
async fn subscribe(
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
		let mut subscriptions = subscriptions.active_subscriptions.lock().unwrap();
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
		if subscriptions.active_subscriptions.lock().unwrap().get(&event).is_none() {
			tracing::info!("tauri watch end: {event}");
			return Ok(());
		}
	}
	Ok(())
}

#[tauri::command]
fn unsubscribe(subscriptions: tauri::State<'_, Subscriptions>, co: CoId, core: Option<&str>, source: &str) {
	let event = build_event_name(co, core);
	let mut subscriptions = subscriptions.active_subscriptions.lock().unwrap();
	// remove subscribers for event type
	if let Some(mut subscribers) = subscriptions.remove(&event) {
		subscribers.remove(source);
		// add list back if other subscribers are remaining
		if subscribers.len() > 0 {
			subscriptions.insert(event, subscribers);
		}
	}
}

pub async fn tauri_builder(co_settings: CoSettings) -> tauri::Builder<Wry> {
	let application = application(co_settings).await;

	tauri::async_runtime::set(tokio::runtime::Handle::current());

	tauri::Builder::default()
		.plugin(tauri_plugin_shell::init())
		.manage(application)
		.manage(Subscriptions { active_subscriptions: Default::default() })
		.invoke_handler(tauri::generate_handler![get_core_state, push, subscribe, unsubscribe])
}
