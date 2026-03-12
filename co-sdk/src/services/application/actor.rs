// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::{epics::epic, Action, ApplicationMessage};
use crate::{
	application::{application::ApplicationSettings, co_context::CoContextInner},
	services::{reducers::ReducersActor, runtime::RuntimeActor},
	CoContext, Cores, DynamicCoAccessPolicy, DynamicCoUuid, DynamicContactHandler, DynamicLocalSecret, Guards, Runtime,
	Storage,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, EpicRuntime, ResponseStreams, TaskSpawner};
use co_identity::LocalIdentityResolver;
use co_primitives::{tags, DynamicCoDate, Tags};
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
#[async_trait]
impl Actor for Application {
	type Message = ApplicationMessage;
	type State = ApplicationState;
	type Initialize = (
		Storage,
		TaskSpawner,
		DynamicCoDate,
		DynamicCoUuid,
		Cores,
		Guards,
		Option<DynamicLocalSecret>,
		Option<DynamicCoAccessPolicy>,
		Option<DynamicContactHandler>,
	);

	async fn initialize(
		&self,
		handle: &ActorHandle<Self::Message>,
		tags: &Tags,
		(storage, spawner, date, uuid, cores, guards, local_secret, co_access_policy, contact_handler): Self::Initialize,
	) -> Result<Self::State, ActorError> {
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
			guards,
			local_secret,
			co_access_policy,
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
