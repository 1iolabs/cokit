// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{Co, CoPrivateIdentity, CoSettings, CoState};
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, Response};
use co_sdk::{
	state, Application, ApplicationBuilder, CoContext, CoId, CoReducerFactory, CoTryStreamExt, CreateCo, Did,
	DidKeyIdentity, DidKeyProvider, PrivateIdentity, PrivateIdentityResolver, Tags, TaskSpawner, CO_CORE_NAME_KEYSTORE,
	CO_ID_LOCAL,
};
use futures::{StreamExt, TryStreamExt};
use std::{future::ready, path::PathBuf};

#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(ignore))]
pub enum CoMessage {
	BasePath(Response<Option<String>>),
	CoOpen(CoId, Response<Result<Co, anyhow::Error>>),
	CoCreate(CoPrivateIdentity, CreateCo, Response<Result<Co, anyhow::Error>>),
	ResolvePrivateIdentity(Did, Response<Result<CoPrivateIdentity, anyhow::Error>>),
	EnsureDidKeyIdentity(String, Response<Result<CoPrivateIdentity, anyhow::Error>>),
	#[cfg(feature = "frb")]
	CoSubscribe(Co, tokio_util::sync::CancellationToken, crate::frb_generated::StreamSink<crate::CoState>),
}

/// CoApplication actor that spawns a Application in a new thread.
#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(ignore))]
#[derive(Debug, Default)]
pub struct CoApplication {}
#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(ignore))]
impl CoApplication {
	pub async fn spawn(settings: CoSettings) -> Result<ActorHandle<CoMessage>, anyhow::Error> {
		let (tx, rx) = tokio::sync::oneshot::channel::<Result<ActorHandle<CoMessage>, anyhow::Error>>();
		std::thread::spawn(|| {
			tokio::runtime::Builder::new_multi_thread()
				.enable_all()
				.build()
				.unwrap()
				.block_on(async move {
					match Actor::spawn(Default::default(), CoApplication::default(), settings) {
						Ok(application) => {
							tx.send(Ok(application.handle())).ok();
							application.join().await.expect("app");
						},
						Err(err) => {
							tx.send(Err(err.into())).ok();
						},
					}
				});
		});
		rx.await?
	}
}
#[async_trait]
impl Actor for CoApplication {
	type Message = CoMessage;
	type State = Application;
	type Initialize = CoSettings;

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &Tags,
		settings: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		let mut application_builder = match settings.path {
			Some(path) => ApplicationBuilder::new_with_path(settings.identifier, PathBuf::from(&path)),
			None => ApplicationBuilder::new(settings.identifier),
		};
		if !settings
			.no_log
			.unwrap_or_else(|| CoSettings::default().no_log.unwrap_or_default())
		{
			application_builder = application_builder.with_bunyan_logging(None);
		}
		if settings
			.no_keychain
			.unwrap_or_else(|| CoSettings::default().no_keychain.unwrap_or_default())
		{
			application_builder = application_builder.without_keychain();
		}
		if settings
			.no_default_features
			.unwrap_or_else(|| CoSettings::default().no_default_features.unwrap_or_default())
		{
			application_builder = application_builder.with_setting("default-features", false);
		}
		application_builder = application_builder.with_log_max_level(settings.log_level.unwrap_or_default().into());
		for feature in settings
			.feature
			.unwrap_or_else(|| CoSettings::default().feature.unwrap_or_default())
		{
			application_builder = application_builder.with_setting("feature", feature.to_owned());
		}
		application_builder = application_builder.with_optional_tracing();
		let mut application = application_builder.build().await?;

		// network
		#[cfg(feature = "network")]
		if settings
			.network
			.unwrap_or_else(|| CoSettings::default().network.unwrap_or_default())
		{
			application
				.create_network(settings.network_settings.unwrap_or_default().try_into()?)
				.await?;
		};

		Ok(application)
	}

	async fn handle(
		&self,
		handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		match message {
			CoMessage::BasePath(response) => {
				response.respond(base_path(state));
			},
			CoMessage::CoOpen(co_id, response) => response.spawn({
				let handle = handle.clone();
				let co_context = state.co().clone();
				move || co_open(co_context, handle, co_id)
			}),
			CoMessage::CoCreate(creator, create, response) => response.spawn({
				let handle = handle.clone();
				let application = state.clone();
				move || co_create(application, handle, creator, create)
			}),
			CoMessage::ResolvePrivateIdentity(did, response) => response.spawn({
				let co_context = state.co().clone();
				move || resolve_private_identity(co_context, did)
			}),
			CoMessage::EnsureDidKeyIdentity(name, response) => response.spawn({
				let co_context = state.co().clone();
				move || ensure_did_key_identity(co_context, name)
			}),
			CoMessage::CoSubscribe(co, cancel, sink) => co_subscribe(state.context().tasks(), co, cancel, sink),
		}
		Ok(())
	}
}

fn base_path(application: &Application) -> Option<String> {
	application
		.settings()
		.base_path()
		.map(|path| path.to_string_lossy().to_string())
}

async fn co_open(co_context: CoContext, handle: ActorHandle<CoMessage>, co_id: CoId) -> Result<Co, anyhow::Error> {
	let co = co_context.try_co_reducer(&co_id).await?;
	Ok(Co::from((handle.into(), co)))
}

async fn co_create(
	application: Application,
	handle: ActorHandle<CoMessage>,
	creator: CoPrivateIdentity,
	create: CreateCo,
) -> Result<Co, anyhow::Error> {
	let co = application.create_co(creator.identity.clone(), create).await?;
	Ok(Co::from((handle.into(), co)))
}

fn co_subscribe(
	tasks: TaskSpawner,
	co: Co,
	cancel: tokio_util::sync::CancellationToken,
	sink: crate::frb_generated::StreamSink<crate::CoState>,
) {
	let stream = co.co.reducer_state_stream().map(CoState::from);
	let task = async move {
		futures::pin_mut!(stream);
		while let Some(item) = stream.next().await {
			if sink.add(item).is_err() {
				break;
			}
		}
	};
	tasks.spawn(async move {
		tokio::select! {
			_ = cancel.cancelled() => {},
			_ = task => {},
		}
	});
}

async fn resolve_private_identity(co_context: CoContext, did: Did) -> Result<CoPrivateIdentity, anyhow::Error> {
	let private_identity_resolver = co_context.private_identity_resolver().await?;
	let private_identity = private_identity_resolver.resolve_private(&did).await?;
	Ok(CoPrivateIdentity::from(private_identity))
}

async fn ensure_did_key_identity(co_context: CoContext, name: String) -> Result<CoPrivateIdentity, anyhow::Error> {
	let local_co = co_context.try_co_reducer(&CoId::new(CO_ID_LOCAL)).await?;
	let storage = local_co.storage();
	let identity = state::identities(storage, local_co.co_state().await, None)
		.try_filter(|identity| ready(identity.name == name && identity.did.starts_with("did:key:")))
		.try_first()
		.await?;
	let private_identity = if let Some(identity) = identity {
		// get
		co_context
			.private_identity_resolver()
			.await?
			.resolve_private(&identity.did)
			.await?
	} else {
		// create
		let identity = DidKeyIdentity::generate(None);
		let provider = DidKeyProvider::new(local_co, CO_CORE_NAME_KEYSTORE);
		provider.store(&identity, Some(name.clone())).await?;

		// result
		identity.boxed()
	};
	Ok(CoPrivateIdentity::from(private_identity))
}
