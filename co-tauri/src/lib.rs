use anyhow::anyhow;
use co_core_membership::Memberships;
use co_sdk::{Application, ApplicationBuilder, CoId, CoReducerError};
use library::co_settings::CoSettings;
use tauri::{ipc::InvokeError, Wry};

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

impl From<CoReducerError> for CoTauriError {
	fn from(value: CoReducerError) -> Self {
		Self { error: anyhow::Error::from(value) }
	}
}

#[tauri::command]
async fn tmp_test_command(application: tauri::State<'_, Application>, name: String) -> Result<String, CoTauriError> {
	let local_reducer = application.local_co_reducer().await?;
	let membership_state = local_reducer.state::<Memberships>("membership").await?;
	Ok(format!(
		"Hello, {}! You've been greeted from Rust! Memberships state for testing: {:#?}",
		name, membership_state
	))
}

#[tauri::command]
async fn get_core_state(
	application: tauri::State<'_, Application>,
	co: CoId,
	core: String,
) -> Result<Memberships, CoTauriError> {
	// TODO this currently only works with the membership core. Working on a solution to dynamically get correct core
	// state.
	let reducer = application
		.co_reducer(co.clone())
		.await?
		.ok_or(anyhow!("Co not found: {}", co.clone()))?;
	let state = reducer.state(&core).await?;
	Ok(state)
}

pub async fn tauri_builder(co_settings: CoSettings) -> tauri::Builder<Wry> {
	let application = application(co_settings).await;

	tauri::async_runtime::set(tokio::runtime::Handle::current());

	tauri::Builder::default()
		.plugin(tauri_plugin_shell::init())
		.manage(application)
		.invoke_handler(tauri::generate_handler![tmp_test_command, get_core_state])
}
