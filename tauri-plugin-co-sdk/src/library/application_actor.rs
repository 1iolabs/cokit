use super::co_application::{application, CoApplicationSettings};
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, Response, ResponseStream};
use co_sdk::{Action, Application, ApplicationMessage, BlockStorage, BlockStorageExt, CoId, CoReducerFactory, Tags};
use futures::{pin_mut, StreamExt};
use libipld::{Block, Cid, DefaultParams, Ipld};
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
	Push(CoId, String, Ipld, Response<Result<Option<Cid>, ActorError>>),
	ResolveCid(CoId, Cid, Response<Result<Ipld, ActorError>>),
}

#[async_trait]
impl Actor for ApplicationActor {
	type Message = ApplicationActorMessage;

	type State = ApplicytionActorState;

	type Initialize = CoApplicationSettings;

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: Tags,
		initialize: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		Ok(ApplicytionActorState { application: application(initialize).await })
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
							.await)
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
								Ok(Action::CoreAction { co, context: _, action: _, cid: _ }) => Some(co),
								_ => None,
							})
						});
						pin_mut!(changed);
						let context = application.context();
						while let Some(co) = changed.next().await {
							if let Some(reducer) = context.try_co_reducer(&co).await.ok() {
								let (state, heads) = reducer.reducer_state().await;
								if response.send((co, state, heads)).is_err() {
									break;
								}
							}
						}
					}
				});
			},
			ApplicationActorMessage::Push(co, core, action, response) => {
				response
					.execute(|| async {
						state
							.application
							.context()
							.try_co_reducer(&co)
							.await
							.map_err(|err| ActorError::Actor(err.into()))?
							.push(&state.application.local_identity(), &core, &action)
							.await
							.map_err(|err| ActorError::Actor(err.into()))
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
		}

		Ok(())
	}
}
