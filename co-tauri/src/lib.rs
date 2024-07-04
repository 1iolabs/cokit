use anyhow::anyhow;
use co_api::Cid;
use co_sdk::{Application, ApplicationBuilder, CoId};
use libipld::{cbor::DagCborCodec, codec::Codec, Ipld};
use library::co_settings::CoSettings;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
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
fn tmp_test_command(application: tauri::State<'_, Application>, name: String) -> String {
	let identifier = &application.settings().identifier;
	format!("Hello, {}! You've been greeted from Rust! App id: {:#?}", name, identifier)
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
	reducer.push(&identity, &*body.core, &body.action).await?;
	Ok(())
}

#[tauri::command]
async fn subscribe(
	application: tauri::State<'_, Application>,
	app: tauri::AppHandle,
	co: CoId,
) -> Result<(), CoTauriError> {
	tracing::info!("tauri command subscribe: {:#?}", co);
	let co_reducer = application
		.co_reducer(co.clone())
		.await?
		.ok_or(anyhow!("Co not found: {}", co))?;

	let mut watcher = WatchStream::from_changes(co_reducer.watch().await);
	while let Some(item) = watcher.next().await {
		match item {
			Some((cid, heads)) => {
				app.emit("test", (cid, heads)).unwrap();
			},
			None => (),
		};
	}
	Ok(())
}

pub async fn tauri_builder(co_settings: CoSettings) -> tauri::Builder<Wry> {
	let application = application(co_settings).await;

	tauri::async_runtime::set(tokio::runtime::Handle::current());

	tauri::Builder::default()
		.plugin(tauri_plugin_shell::init())
		.manage(application)
		.invoke_handler(tauri::generate_handler![tmp_test_command, get_core_state, push, subscribe])
}
