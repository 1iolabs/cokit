use super::{epics::epic, Action, ApplicationMessage};
use crate::{
	application::{application::ApplicationSettings, co_context::CoContextInner},
	services::reducers::ReducersActor,
	CoContext, DynamicCoDate, DynamicCoUuid, Network, Runtime, Storage,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, ActorInstance, EpicRuntime, ResponseStreams, TaskSpawner};
use co_identity::LocalIdentityResolver;
use co_primitives::{tags, Tags};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

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
	type Initialize = (Storage, TaskTracker, DynamicCoDate, DynamicCoUuid);

	async fn initialize(
		&self,
		handle: &ActorHandle<Self::Message>,
		tags: &Tags,
		(storage, tasks, date, uuid): Self::Initialize,
	) -> Result<Self::State, ActorError> {
		tracing::trace!(settings = ?self.settings, "application-initialize");

		let shutdown = CancellationToken::new();
		let local_identity = LocalIdentityResolver::default().private_identity("did:local:device").unwrap();
		let runtime = Runtime::new();

		// reducers
		let reducers = Actor::spawner(
			tags!("type": "reducers", "application": self.settings.identifier.clone()),
			ReducersActor::new(),
		)?;

		// co
		let co_context: CoContext = CoContextInner::new(
			self.settings.clone(),
			shutdown.child_token(),
			TaskSpawner::new(self.settings.identifier.clone(), tasks.clone()),
			local_identity.clone(),
			None,
			storage.storage(),
			storage.tmp_storage(),
			runtime.clone(),
			handle.clone(),
			reducers.handle().into(),
			date,
			uuid,
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
			network: None,
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
			ApplicationMessage::Dispatch(Action::NetworkStart { force_new_peer_id }) => {
				if state.network.is_none() {
					state.network = Some(Actor::spawn_with(
						state.context.tasks(),
						tags!("type": "network", "application": &self.settings.identifier),
						Network::new(state.context.clone(), force_new_peer_id),
						(),
					)?);
				}
				Some(Action::NetworkStart { force_new_peer_id })
			},
			ApplicationMessage::Dispatch(action) => Some(action),
			ApplicationMessage::Subscribe(response) => {
				state.subscriptions.push(response);
				None
			},
			ApplicationMessage::Context(response) => {
				response.send(state.context.clone()).ok();
				None
			},
			ApplicationMessage::Network(response) => {
				if let Some(network) = &state.network {
					response.send(Ok(network.handle())).ok();
				} else {
					response.send(Err(anyhow!("Not started"))).ok();
				}
				None
			},
		};

		// epic
		if let Some(action) = &action {
			state.epic.handle(&handle, action, &(), &state.context);
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
	network: Option<ActorInstance<Network>>,
}
