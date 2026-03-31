// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::{epics::epic, Action, ApplicationMessage};
use crate::{
	application::{application::ApplicationSettings, co_context::CoContextInner},
	services::reducers::ReducersActor,
	CoContext, Cores, DynamicCoUuid, DynamicContactHandler, DynamicLocalSecret, Runtime, Storage,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, EpicRuntime, ResponseStreams, TaskSpawner};
#[cfg(feature = "guard")]
use co_guard::{DynamicAccessGuard, Guards};
use co_identity::LocalIdentityResolver;
use co_primitives::{tags, DynamicCoDate, Tags};
use co_runtime::RuntimeActor;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct Application {
	settings: ApplicationSettings,
}
impl Application {
	pub fn new(settings: ApplicationSettings) -> Self {
		Self { settings }
	}
}
pub struct ApplicationInitialize {
	pub storage: Storage,
	pub tasks: TaskSpawner,
	pub date: DynamicCoDate,
	pub uuid: DynamicCoUuid,
	pub cores: Cores,
	#[cfg(feature = "guard")]
	pub guards: Guards,
	pub local_secret: Option<DynamicLocalSecret>,
	#[cfg(feature = "guard")]
	pub access_guard: Option<DynamicAccessGuard>,
	pub contact_handler: Option<DynamicContactHandler>,
}

#[async_trait]
impl Actor for Application {
	type Message = ApplicationMessage;
	type State = ApplicationState;
	type Initialize = ApplicationInitialize;

	async fn initialize(
		&self,
		handle: &ActorHandle<Self::Message>,
		tags: &Tags,
		init: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		let ApplicationInitialize {
			storage,
			tasks: spawner,
			date,
			uuid,
			cores,
			#[cfg(feature = "guard")]
			guards,
			local_secret,
			#[cfg(feature = "guard")]
				access_guard: co_access_guard,
			contact_handler,
		} = init;

		tracing::trace!(settings = ?self.settings, "application-initialize");
		let shutdown = CancellationToken::new();
		let local_identity = LocalIdentityResolver::default().private_identity("did:local:device").unwrap();

		// service: runtime
		#[cfg(feature = "js")]
		let runtime = Runtime::new(RuntimeActor::spawn_local(self.settings.identifier.clone())?);
		#[cfg(not(feature = "js"))]
		let runtime = Runtime::new(RuntimeActor::spawn(self.settings.identifier.clone(), spawner.clone())?);

		// service: reducers
		let reducers = Actor::spawner(
			tags!("type": "reducers", "application": self.settings.identifier.clone()),
			ReducersActor::new(),
		)?;

		// co
		let co_context: CoContext = CoContextInner::new(
			self.settings.clone(),
			shutdown.child_token(),
			spawner.clone(),
			local_identity.clone(),
			#[cfg(feature = "network")]
			None,
			storage,
			runtime.clone(),
			handle.clone(),
			reducers.handle().into(),
			date,
			uuid,
			cores,
			#[cfg(feature = "guard")]
			guards,
			local_secret,
			#[cfg(feature = "guard")]
			co_access_guard,
			contact_handler,
		)
		.into();

		// reducers
		reducers.spawn(co_context.tasks(), co_context.clone());

		// result
		Ok(ApplicationState {
			epic: EpicRuntime::new(epic(tags.clone()), |err| {
				tracing::error!(?err, "application-epic-error");
				Some(Action::Error { err: err.into() })
			}),
			subscriptions: Default::default(),
			context: co_context,
		})
	}

	async fn handle(
		&self,
		handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		// handle
		let action = match message {
			ApplicationMessage::Dispatch(action) => Some(action),
			ApplicationMessage::Subscribe(response) => {
				state.subscriptions.push(response);
				None
			},
			ApplicationMessage::Context(response) => {
				response.send(state.context.clone()).ok();
				None
			},
			#[cfg(feature = "network")]
			ApplicationMessage::Network(response) => {
				response.respond(state.context.network().await.ok_or(anyhow!("Not started")));
				None
			},
		};

		// epic
		if let Some(action) = &action {
			state.epic.handle(&state.context.tasks(), handle, action, &(), &state.context);
		}

		// responses
		if let Some(action) = action {
			state.subscriptions.send(action);
		}

		// result
		Ok(())
	}

	async fn shutdown(&self, state: Self::State) -> Result<(), ActorError> {
		state.context.inner.shutdown().cancel();
		state.context.inner.reducers_control().handle.shutdown();
		Ok(())
	}
}

#[derive(Debug)]
pub struct ApplicationState {
	epic: EpicRuntime<ApplicationMessage, Action, (), CoContext>,
	context: CoContext,
	subscriptions: ResponseStreams<Action>,
}
