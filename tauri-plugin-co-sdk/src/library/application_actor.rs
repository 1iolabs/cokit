use crate::library::co_application::{start_application, CoApplicationSettings};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_actor::{Actor, ActorError, ActorHandle, Response, ResponseStream};
use co_primitives::{Block, DefaultParams};
use co_sdk::{
	Action, Application, ApplicationMessage, BlockStorage, BlockStorageExt, CoId, CoReducer, CoReducerFactory, Did,
	DidKeyIdentity, DidKeyProvider, Identity, PrivateIdentityResolver, Tags, CO_CORE_NAME_KEYSTORE,
};
use futures::{pin_mut, StreamExt};
use ipld_core::ipld::Ipld;
use serde::{Deserialize, Serialize};
use std::{
	collections::{BTreeSet, HashMap},
	fmt::Display,
	future::ready,
	hash::Hash,
};

#[derive(Default, Debug)]
pub struct ApplicationActor {}

#[derive(Clone)]
pub struct Session {
	reducer: CoReducer,
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Hash, Clone, Debug)]
pub struct SessionId(String);

impl Display for SessionId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

impl From<&String> for SessionId {
	fn from(value: &String) -> Self {
		SessionId(value.to_string())
	}
}
impl Default for SessionId {
	fn default() -> Self {
		Self::new()
	}
}
impl SessionId {
	pub fn new() -> Self {
		SessionId(uuid::Uuid::new_v4().into())
	}
}

pub struct ApplicationActorState {
	application: Application,
	sessions: HashMap<SessionId, Session>,
}

#[derive(Debug)]
pub enum ApplicationActorMessage {
	SessionOpen(CoId, Response<Result<SessionId, anyhow::Error>>),
	SessionClose(SessionId),
	StorageGet(SessionId, Cid, Response<Result<Block<DefaultParams>, anyhow::Error>>),
	StorageSet(SessionId, Block<DefaultParams>, Response<Result<Cid, anyhow::Error>>),
	GetCoState(CoId, Response<Result<(Option<Cid>, BTreeSet<Cid>), anyhow::Error>>),
	WatchState(ResponseStream<(CoId, Option<Cid>, BTreeSet<Cid>)>),
	Push(SessionId, String, Ipld, Did, Response<Result<Option<Cid>, anyhow::Error>>),
	ResolveCid(SessionId, Cid, Response<Result<Ipld, anyhow::Error>>),
	GetActions(GetActionsRequest, Response<Result<GetActionsResponse, anyhow::Error>>),
	CreateIdentity(CreateIdentityRequest, Response<Result<Did, anyhow::Error>>),
	CreateCo(CreateCoRequest),
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetActionsRequest {
	pub session: SessionId,
	pub heads: BTreeSet<Cid>,
	pub count: usize,
	pub until: Option<Cid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetActionsResponse {
	pub actions: Vec<Cid>,
	pub next_heads: BTreeSet<Cid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateIdentityRequest {
	pub name: String,
	pub seed: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCoRequest {
	pub creator_did: String,
	pub co_id: CoId,
	pub co_name: String,
	pub public: bool,
}

#[derive(Debug, derive_more::From)]
pub enum ApplicationActorInizialize {
	Settings(CoApplicationSettings),
	App(Application),
}

#[async_trait]
impl Actor for ApplicationActor {
	type Message = ApplicationActorMessage;

	type State = ApplicationActorState;

	type Initialize = ApplicationActorInizialize;

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &Tags,
		initialize: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		let app = match initialize {
			ApplicationActorInizialize::Settings(co_application_settings) => {
				start_application(co_application_settings).await?
			},
			ApplicationActorInizialize::App(application) => application,
		};
		Ok(ApplicationActorState { application: app, sessions: HashMap::new() })
	}

	async fn handle(
		&self,
		_handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		tracing::debug!(name: "application_handle", "actor handle function called with message {:#?}", message);
		match message {
			ApplicationActorMessage::SessionOpen(co_id, response) => {
				response
					.respond_execute(|| async {
						let reducer = state
							.application
							.context()
							.try_co_reducer(&co_id)
							.await
							.map_err(|err| ActorError::Actor(err.into()))?;
						let session_id = SessionId::new();
						state.sessions.insert(session_id.clone(), Session { reducer });
						Ok(session_id)
					})
					.await;
			},
			ApplicationActorMessage::SessionClose(session_id) => {
				state.sessions.remove(&session_id);
			},
			ApplicationActorMessage::StorageGet(session_id, cid, response) => {
				let sessions = state.sessions.clone();
				response.spawn(move || async move {
					let session = sessions
						.get(&session_id)
						.ok_or(ActorError::Actor(anyhow!("Session not found: No session for ID {session_id}")))?;
					Ok(session
						.reducer
						.storage()
						.get(&cid)
						.await
						.map_err(|err| ActorError::Actor(err.into()))?)
				});
			},
			ApplicationActorMessage::StorageSet(session_id, block, response) => {
				let sessions = state.sessions.clone();
				response.spawn(move || async move {
					let session = sessions
						.get(&session_id)
						.ok_or(ActorError::Actor(anyhow!("Session not found: No session for ID {session_id}")))?;
					Ok(session
						.reducer
						.storage()
						.set(block)
						.await
						.map_err(|err| ActorError::Actor(err.into()))?)
				});
			},
			ApplicationActorMessage::GetCoState(co, response) => {
				response
					.execute(|| async {
						Ok(state
							.application
							.context()
							.try_co_reducer(&co)
							.await
							.map_err(|err| ActorError::Actor(err.into()))?
							.reducer_state()
							.await
							.into())
					})
					.await
					.ok();
			},
			ApplicationActorMessage::WatchState(mut response) => {
				state.application.context().tasks().spawn({
					let application = state.application.clone();
					async move {
						let changed = application.handle().stream(ApplicationMessage::Subscribe).filter_map(|action| {
							ready(match action {
								// Ok(Action::CoreAction { co, storage: _, context: _, action: _, cid: _, head: _ }) =>
								// { 	Some(co)
								// },
								// Ok(Action::Invite { co, from: _, to: _ }) => Some(co),
								// Ok(Action::InviteSent { co, to: _, peer: _ }) => Some(co),
								// Ok(Action::JoinKeyRequest { co, participant: _, peer: _ }) => Some(co),
								// Ok(Action::Joined { co, participant: _, success: _, peer: _ }) => Some(co),
								Ok(Action::CoFlush { co, info: _ }) => Some(co),
								_ => None,
							})
						});
						pin_mut!(changed);
						let context = application.context();
						while let Some(co) = changed.next().await {
							if let Ok(reducer) = context.try_co_reducer(&co).await {
								let (state, heads) = reducer.reducer_state().await.into();
								tracing::debug!("tauri watch: new state: {:?}, {:?}", co, state);
								if response.send((co, state, heads)).is_err() {
									break;
								}
							}
						}
					}
				});
			},
			ApplicationActorMessage::Push(session_id, core, action, identity, response) => {
				let sessions = state.sessions.clone();
				response
					.execute(|| async {
						let session = sessions
							.get(&session_id)
							.ok_or(anyhow!("Session not found: No session for ID {session_id}"))?;
						let private_identity = state.application.private_identity(&identity).await?;
						session
							.reducer
							.push(&private_identity, &core, &action)
							.await
							.map(|state| state.state())
					})
					.await
					.ok();
			},
			ApplicationActorMessage::ResolveCid(session_id, cid, response) => {
				let sessions = state.sessions.clone();
				response.spawn(move || async move {
					let session = sessions
						.get(&session_id)
						.ok_or(ActorError::Actor(anyhow!("Session not found: No session for ID {session_id}")))?;
					Ok(session
						.reducer
						.storage()
						.get_deserialized::<Ipld>(&cid)
						.await
						.map_err(|err| ActorError::Actor(err.into()))?)
				});
			},
			ApplicationActorMessage::GetActions(request, response) => {
				let sessions = state.sessions.clone();
				let mut next_heads = request.heads.clone();
				let co_context = state.application.context().clone();
				response.spawn(move || async move {
					let session_id = request.session;
					let session = sessions
						.get(&session_id)
						.ok_or(ActorError::Actor(anyhow!("Session not found: No session for ID {session_id}")))?;
					let storage = sessions
						.get(&session_id)
						.ok_or(ActorError::Actor(anyhow!("Session not found: No session for ID {session_id}")))?
						.reducer
						.storage();

					let stream = co_context
						.entries_from_heads(session.reducer.id(), storage, request.heads)
						.await?
						.take(request.count);
					pin_mut!(stream);
					let mut actions = Vec::new();
					while let Some(item) = stream.next().await {
						let entry_block = item.map_err(ActorError::Actor)?;

						// resolve entry from block
						let entry_payload = entry_block.entry().payload;

						// check if we reached action we should stop at
						if let Some(until) = request.until {
							if until == entry_payload {
								break;
							}
						}

						actions.push(entry_payload);

						// keep track of heads
						next_heads.remove(entry_block.cid());
						next_heads.append(&mut entry_block.entry().next.clone());
					}
					Ok(GetActionsResponse { actions, next_heads })
				});
			},
			ApplicationActorMessage::CreateIdentity(request, response) => {
				let identity = DidKeyIdentity::generate(request.seed.as_deref());
				let co = state.application.local_co_reducer().await?;
				let provider = DidKeyProvider::new(co, CO_CORE_NAME_KEYSTORE);
				provider.store(&identity, Some(request.name)).await?;
				response.respond(Ok(identity.identity().to_owned()));
			},
			ApplicationActorMessage::CreateCo(request) => {
				// resolve identity
				let identity = state
					.application
					.private_identity_resolver()
					.await?
					.resolve_private(&request.creator_did)
					.await
					.map_err(|err| ActorError::Actor(err.into()))?;

				// create co options
				let create_co = co_sdk::CreateCo::new(request.co_id, Some(request.co_name));

				// create co
				state.application.create_co(identity, create_co).await?;
			},
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use crate::{library::application_actor::GetActionsRequest, ApplicationActor, ApplicationActorMessage};
	use co_actor::Actor;
	use co_core_co::CoAction;
	use co_messaging::{message_event::TextContent, MatrixEvent};
	use co_sdk::{tags, ApplicationBuilder, CoId, CoReducerFactory, Cores, CreateCo, CO_CORE_NAME_CO, CO_CORE_ROOM};

	#[tokio::test]
	async fn test_application() {
		// init app with 1io co and room core
		let co: CoId = "1io".into();
		let app = ApplicationBuilder::new_memory("actor_test".to_owned())
			.without_keychain()
			.build()
			.await
			.expect("app built");
		app.create_co(app.local_identity(), CreateCo::new(co.clone(), Some(co.clone().into())).with_public(true))
			.await
			.expect("co created");
		let reducer = app.co().co_reducer(&co.clone()).await.expect("reducer").unwrap();
		let create = CoAction::CoreCreate {
			core: "room".to_owned(),
			binary: Cores::default().binary(CO_CORE_ROOM).expect(CO_CORE_ROOM),
			tags: tags!("core": CO_CORE_ROOM),
		};
		reducer
			.push(&app.local_identity(), CO_CORE_NAME_CO, &create)
			.await
			.expect("action pushed");

		// fill room core with 10 messages
		for i in 0..10 {
			let new_action: MatrixEvent = MatrixEvent::new(
				format!("event{}", i),
				i,
				format!("event{}", i),
				TextContent::new(format!("event{}", i)),
			);
			reducer
				.push(&app.local_identity(), "room", &new_action)
				.await
				.expect("action pushed");
		}

		// init actor
		let actor_handle = Actor::spawn(Default::default(), ApplicationActor {}, app.into())
			.expect("actor")
			.handle();

		// open a session
		let session_id = actor_handle
			.request(|r| ApplicationActorMessage::SessionOpen("local".into(), r))
			.await
			.expect("created session")
			.expect("created session");

		// get 10 actions at once from current heads
		let heads = reducer.heads().await;
		let mut request = GetActionsRequest { session: session_id.clone(), heads, count: 10, until: None };
		let log_a = actor_handle
			.request(|r| ApplicationActorMessage::GetActions(request.clone(), r))
			.await
			.unwrap()
			.expect("entries")
			.actions;

		// get 5 actions from current heads
		request.count = 5;
		let mut log_response_b = actor_handle
			.request(|r| ApplicationActorMessage::GetActions(request.clone(), r))
			.await
			.unwrap()
			.expect("entries");

		// get another 5 from returned heads
		request.heads = log_response_b.next_heads;
		let mut log_b_next = actor_handle
			.request(|r| ApplicationActorMessage::GetActions(request, r))
			.await
			.unwrap()
			.expect("entries")
			.actions;

		// combine the 5 actions each
		log_response_b.actions.append(&mut log_b_next);
		// should be the same as if we got all 10 at once
		assert_eq!(log_a, log_response_b.actions);

		let (local_co_state_cid, _) = actor_handle
			.request(|r| ApplicationActorMessage::GetCoState("local".into(), r))
			.await
			.unwrap()
			.expect("local co state");

		let local_co_state = actor_handle
			.request(|r| ApplicationActorMessage::ResolveCid(session_id.clone(), local_co_state_cid.expect("state"), r))
			.await
			.expect("local co state")
			.expect("local co state");
		println!("{:#?}", local_co_state);
		actor_handle
			.dispatch(ApplicationActorMessage::SessionClose(session_id))
			.expect("close session");
	}
}
