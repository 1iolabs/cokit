use crate::{Co, CoPrivateIdentity, CoSettings};
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, Response};
use co_sdk::{Application, ApplicationBuilder, CoContext, CoId, CoReducerFactory, Did, PrivateIdentityResolver, Tags};
use std::path::PathBuf;

#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(ignore))]
pub enum CoMessage {
	OpenCo(CoId, Response<Result<Co, anyhow::Error>>),
	ResolvePrivateIdentity(Did, Response<Result<CoPrivateIdentity, anyhow::Error>>),
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
		application_builder = application_builder.with_setting("feature", "co-open-keep");
		let mut application = application_builder.build().await?;

		// network
		if settings
			.network
			.unwrap_or_else(|| CoSettings::default().network.unwrap_or_default())
		{
			application
				.create_network(settings.network_settings.unwrap_or_default().try_into()?)
				.await?;
		}

		Ok(application)
	}

	async fn handle(
		&self,
		_handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		match message {
			CoMessage::OpenCo(co_id, response) => response.spawn({
				let co_context = state.co().clone();
				move || open_co(co_context, co_id)
			}),
			CoMessage::ResolvePrivateIdentity(did, response) => response.spawn({
				let co_context = state.co().clone();
				move || resolve_private_identity(co_context, did)
			}),
		}
		Ok(())
	}
}

async fn open_co(co_context: CoContext, co_id: CoId) -> Result<Co, anyhow::Error> {
	let co = co_context.try_co_reducer(&co_id).await?;
	Ok(Co::from(co))
}

async fn resolve_private_identity(co_context: CoContext, did: Did) -> Result<CoPrivateIdentity, anyhow::Error> {
	let private_identity_resolver = co_context.private_identity_resolver().await?;
	let private_identity = private_identity_resolver.resolve_private(&did).await?;
	Ok(CoPrivateIdentity::from(private_identity))
}
