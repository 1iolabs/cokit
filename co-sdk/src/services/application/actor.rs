use super::{epics::epic, Action, ApplicationMessage};
use crate::{
	application::{
		application::ApplicationSettings,
		co_context::{CoContextInner, Reducers},
	},
	CoContext, Network, Runtime, Storage,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, ActorInstance, EpicRuntime, ResponseStreams, TaskSpawner};
use co_identity::LocalIdentityResolver;
use co_primitives::{tags, Tags};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

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
	type Initialize = (Storage, TaskTracker);

	async fn initialize(
		&self,
		handle: &ActorHandle<Self::Message>,
		tags: Tags,
		(storage, tasks): Self::Initialize,
	) -> Result<Self::State, ActorError> {
		let shutdown = CancellationToken::new();
		let local_identity = LocalIdentityResolver::default().private_identity("did:local:device").unwrap();
		let runtime = Runtime::new();

		// reducers
		let (reducers, reducers_control) = Reducers::new();

		// co
		let co_context: CoContext = CoContextInner::new(
			self.settings.clone(),
			shutdown.child_token(),
			TaskSpawner::new(self.settings.identifier.clone(), tasks.clone()),
			local_identity.clone(),
			None,
			storage.storage(),
			runtime.clone(),
			handle.clone(),
			reducers_control,
		)
		.into();

		// reducers
		co_context.tasks().spawn(reducers.worker(co_context.inner.clone()));

		// reuslt
		Ok(ApplicationState {
			epic: EpicRuntime::new(epic(tags), |err| {
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
}

pub struct ApplicationState {
	epic: EpicRuntime<ApplicationMessage, Action, (), CoContext>,
	context: CoContext,
	subscriptions: ResponseStreams<Action>,
	network: Option<ActorInstance<Network>>,
}
