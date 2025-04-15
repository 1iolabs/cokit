use async_trait::async_trait;
use cid::Cid;
use co_actor::{Actor, ActorError, ActorHandle, Response, ResponseStream};
use co_primitives::{Block, DefaultParams};
use co_sdk::{
	Action, Application, ApplicationMessage, BlockStorage, BlockStorageExt, CoId, CoReducerFactory, Did,
	DidKeyIdentity, DidKeyProvider, Tags, CO_CORE_NAME_KEYSTORE,
};
use futures::{pin_mut, StreamExt};
use ipld_core::ipld::Ipld;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, future::ready};

pub struct ApplicationActor {}
pub struct ApplicytionActorState {
	application: Application,
}
pub enum ApplicationActorMessage {
	StorageGet(CoId, Cid, Response<Result<Block<DefaultParams>, ActorError>>),
	StorageSet(CoId, Block<DefaultParams>, Response<Result<Cid, ActorError>>),
	GetCoState(CoId, Response<Result<(Option<Cid>, BTreeSet<Cid>), ActorError>>),
	WatchState(ResponseStream<(CoId, Option<Cid>, BTreeSet<Cid>)>),
	Push(CoId, String, Ipld, Did, Response<Result<Option<Cid>, ActorError>>),
	ResolveCid(CoId, Cid, Response<Result<Ipld, ActorError>>),
	GetActions(GetActionsRequest, Response<Result<GetActionsResponse, ActorError>>),
	CreateIdentity(CreateIdentityRequest),
}

#[derive(Debug, Clone)]
pub struct GetActionsRequest {
	pub co: CoId,
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

#[async_trait]
impl Actor for ApplicationActor {
	type Message = ApplicationActorMessage;

	type State = ApplicytionActorState;

	type Initialize = Application;

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &Tags,
		initialize: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		Ok(ApplicytionActorState { application: initialize })
	}

	async fn handle(
		&self,
		_handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		match message {
			ApplicationActorMessage::StorageGet(co_id, cid, response) => {
				let context = state.application.context().clone();
				response.spawn(move || async move {
					Ok(context
						.try_co_reducer(&co_id)
						.await
						.map_err(|err| ActorError::Actor(err.into()))?
						.storage()
						.get(&cid)
						.await
						.map_err(|err| ActorError::Actor(err.into()))?)
				});
			},
			ApplicationActorMessage::StorageSet(co, block, response) => {
				let context = state.application.context().clone();
				response.spawn(move || async move {
					Ok(context
						.try_co_reducer(&co)
						.await
						.map_err(|err| ActorError::Actor(err.into()))?
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
								Ok(Action::CoreAction { co, storage: _, context: _, action: _, cid: _ }) => Some(co),
								_ => None,
							})
						});
						pin_mut!(changed);
						let context = application.context();
						while let Some(co) = changed.next().await {
							if let Some(reducer) = context.try_co_reducer(&co).await.ok() {
								let (state, heads) = reducer.reducer_state().await.into();
								if response.send((co, state, heads)).is_err() {
									break;
								}
							}
						}
					}
				});
			},
			ApplicationActorMessage::Push(co, core, action, identity, response) => {
				response
					.execute(|| async {
						let private_identity = state
							.application
							.private_identity(&identity)
							.await
							.map_err(|err| ActorError::Actor(err.into()))?;
						state
							.application
							.context()
							.try_co_reducer(&co)
							.await
							.map_err(|err| ActorError::Actor(err.into()))?
							.push(&private_identity, &core, &action)
							.await
							.map_err(|err| ActorError::Actor(err.into()))
							.map(|state| state.state())
					})
					.await
					.ok();
			},
			ApplicationActorMessage::ResolveCid(co, cid, response) => {
				let context = state.application.context().clone();
				response.spawn(move || async move {
					Ok(context
						.try_co_reducer(&co)
						.await
						.map_err(|err| ActorError::Actor(err.into()))?
						.storage()
						.get_deserialized::<Ipld>(&cid)
						.await
						.map_err(|err| ActorError::Actor(err.into()))?)
				});
			},
			ApplicationActorMessage::GetActions(request, response) => {
				let context = state.application.context().clone();
				let mut next_heads = request.heads.clone();
				response.spawn(move || async move {
					let storage = context.co_reducer(&request.co).await?.unwrap().storage();
					let stream = co_log::create_stream(&storage, request.heads).take(request.count);
					pin_mut!(stream);
					let mut actions = Vec::new();
					while let Some(item) = stream.next().await {
						let entry_block = item.map_err(|err| ActorError::Actor(err.into()))?;

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
			ApplicationActorMessage::CreateIdentity(request) => {
				let identity = DidKeyIdentity::generate(request.seed.as_deref());
				let co = state.application.local_co_reducer().await?;
				let provider = DidKeyProvider::new(co, CO_CORE_NAME_KEYSTORE);
				provider.store(&identity, Some(request.name)).await?;
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
		app.create_co(app.local_identity(), CreateCo { id: co.clone(), name: co.clone().into(), algorithm: None })
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
		let actor_handle = Actor::spawn(Default::default(), ApplicationActor {}, app)
			.expect("actor")
			.handle();

		// get 10 actions at once from current heads
		let heads = reducer.heads().await;
		let mut request = GetActionsRequest { co, heads, count: 10, until: None };
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
	}
}
