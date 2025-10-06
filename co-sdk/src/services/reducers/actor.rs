use super::{ReducerRequest, ReducerStorage, ReducersControl};
use crate::{
	library::settings_timeout::settings_timeout,
	types::{co_reducer_context::CoReducerFeature, co_reducer_factory::CoReducerFactoryError},
	Action, CoContext, CoReducer, CO_ID_LOCAL,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle};
use co_primitives::{BlockStorageSettings, CloneWithBlockStorageSettings, CoId, Tags};
use std::collections::{BTreeMap, VecDeque};

pub struct Reducers {
	context: CoContext,
	reducers: BTreeMap<CoId, CoReducer>,
	storages: BTreeMap<CoId, ReducerStorage>,
	pending_requests: VecDeque<ReducerRequest>,
	keep_open: bool,
}
impl Reducers {
	async fn local(&mut self) -> Result<CoReducer, CoReducerFactoryError> {
		let local_id = CoId::from(CO_ID_LOCAL);
		let local = if let Some(local) = self.reducers.get(&local_id) {
			local.clone()
		} else {
			let local = self.context.inner.create_local_co_instance(true).await?;
			self.reducers.insert(local.id().clone(), local.clone());
			local
		};
		Ok(local)
	}

	fn pending_request_count(&self, co: &CoId) -> usize {
		self.pending_requests.iter().fold(0, |a, b| match b {
			ReducerRequest::Request(id, _) if id == co => a + 1,
			_ => a,
		})
	}

	fn pending_storage_count(&self, co: &CoId) -> usize {
		self.pending_requests.iter().fold(0, |a, b| match b {
			ReducerRequest::Storage(id, _) if id == co => a + 1,
			_ => a,
		})
	}
}

pub struct ReducersActor {}
impl ReducersActor {
	pub fn new() -> Self {
		Self {}
	}
}
#[async_trait]
impl Actor for ReducersActor {
	type Message = ReducerRequest;
	type State = Reducers;
	type Initialize = CoContext;

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &Tags,
		context: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		Ok(Reducers {
			keep_open: context.settings().feature_co_open_keep(),
			context,
			reducers: Default::default(),
			storages: Default::default(),
			pending_requests: Default::default(),
		})
	}

	async fn handle(
		&self,
		handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		match message {
			ReducerRequest::Storage(id, response) => {
				// local
				let local = match state.local().await {
					Ok(local) => local,
					Err(err) => {
						response
							.send(Err(CoReducerFactoryError::Create(CoId::from(CO_ID_LOCAL), err.into())))
							.ok();
						return Ok(());
					},
				};

				// get/create
				if let Some(storage) = state.storages.get(&id) {
					response.send(Ok(storage.clone())).ok();
				} else {
					state.pending_requests.push_back(ReducerRequest::Storage(id.clone(), response));
					if state.pending_storage_count(&id) == 1 {
						// create storage
						state.context.tasks().spawn({
							let control: ReducersControl = handle.clone().into();
							let context = state.context.clone();
							let parent = local.clone();
							async move {
								let timeout =
									settings_timeout(&context, &CoId::from(CO_ID_LOCAL), Some("key-request")).await;
								let result = ReducerStorage::from_id(
									context.inner.application(),
									context
										.inner
										.storage()
										.clone_with_settings(BlockStorageSettings::new().with_detached()),
									parent,
									id.clone(),
									timeout,
								)
								.await;
								control.create_storage(id, result).await;
							}
						});
					}
				}
			},
			ReducerRequest::Request(id, response) => {
				// local
				let local = match state.local().await {
					Ok(local) => local,
					Err(err) => {
						response
							.send(Err(CoReducerFactoryError::Create(CoId::from(CO_ID_LOCAL), err.into())))
							.ok();
						return Ok(());
					},
				};

				// get/create
				if let Some(reducer) = state.reducers.get(&id) {
					response
						.send(Ok(co_reducer_instance(&state.context, &reducer, state.keep_open)))
						.ok();
				} else {
					state.pending_requests.push_back(ReducerRequest::Request(id.clone(), response));
					if state.pending_request_count(&id) == 1 {
						// create shared co
						state.context.tasks().spawn({
							let control: ReducersControl = handle.clone().into();
							let context = state.context.clone();
							let parent = local.clone_with_detached_storage();
							async move {
								// get storage
								let result = match control.clone().storage(id.clone()).await {
									Ok(storage) => {
										// create reducer
										match context.inner.create_co_instance(parent, &id, storage, true, None).await {
											Ok(Some(reducer)) => Ok(reducer),
											Ok(None) => Err(CoReducerFactoryError::CoNotFound(
												id.clone(),
												anyhow!("Create retuned None"),
											)),
											Err(err) => Err(CoReducerFactoryError::Create(id.clone(), err)),
										}
									},
									Err(err) => Err(err),
								};

								// notify
								control.clone().create(id, result).await;
							}
						});
					}
				}
			},
			ReducerRequest::RequestOpt(id, wait_on_pending_create, response) => {
				if let Some(reducer) = state.reducers.get(&id) {
					// use already created
					response
						.send(Some(co_reducer_instance(&state.context, &reducer, state.keep_open)))
						.ok();
				} else if wait_on_pending_create && state.pending_request_count(&id) > 0 {
					// wait if create is currently pending
					state.pending_requests.push_back(ReducerRequest::RequestOpt(
						id.clone(),
						wait_on_pending_create,
						response,
					));
				} else {
					// not created and not peding
					response.send(None).ok();
				}
			},
			ReducerRequest::Clear(response) => {
				// remove
				let mut remove = Vec::new();
				state.reducers.retain(|retain_id, _| {
					if retain_id.as_str() == CO_ID_LOCAL {
						true
					} else {
						remove.push(retain_id.clone());
						false
					}
				});

				// notify
				for id in remove {
					state.context.inner.application().dispatch(Action::CoClose { co: id }).ok();
				}

				// response
				response.send(Ok(())).ok();
			},
			ReducerRequest::ClearOne(id, response) => {
				// remove
				let mut remove = Vec::new();
				state.reducers.retain(|retain_id, _| {
					if retain_id == &id {
						remove.push(retain_id.clone());
						false
					} else {
						true
					}
				});

				// notify
				for id in remove {
					state
						.context
						.inner
						.application()
						.dispatch(Action::CoClose { co: id.clone() })
						.ok();
				}

				// response
				response.send(Ok(())).ok();
			},
			ReducerRequest::Create(id, result) => {
				// register
				let notification = match &result {
					Ok(reducer) => {
						// reducer
						//  note: we negate keep_open so when it is set we get an instance with the "global" overlay
						let reducer = co_reducer_instance(&state.context, reducer, !state.keep_open);
						let network = reducer.context.has_feature(&CoReducerFeature::Network);

						// store
						state.reducers.insert(reducer.id().clone(), reducer);

						// result
						Some(Action::CoOpen { co: id.clone(), network })
					},
					Err(err) => {
						tracing::error!(co = ?id, ?err, "co-reducer-failed");
						None
					},
				};

				// respond pending
				let mut remove = state
					.pending_requests
					.iter()
					.enumerate()
					.filter_map(|(index, request)| match request {
						ReducerRequest::Request(request_id, _) if request_id == &id => Some(index),
						ReducerRequest::RequestOpt(request_id, _, _) if request_id == &id => Some(index),
						_ => None,
					})
					.collect::<VecDeque<usize>>();
				while let Some(index) = remove.pop_back() {
					match state.pending_requests.remove(index) {
						Some(ReducerRequest::Request(_, response)) => {
							if remove.is_empty() {
								response
									.send(match result {
										Err(err) => Err(err),
										Ok(reducer) => {
											Ok(co_reducer_instance(&state.context, &reducer, state.keep_open))
										},
									})
									.ok();
								break;
							} else {
								response
									.send(match &result {
										Err(err) => Err(co_reducerfactory_error_clone(err)),
										Ok(reducer) => {
											Ok(co_reducer_instance(&state.context, &reducer, state.keep_open))
										},
									})
									.ok();
							}
						},
						Some(ReducerRequest::RequestOpt(_, _, response)) => {
							response
								.send(match &result {
									Err(_err) => None,
									Ok(reducer) => Some(co_reducer_instance(&state.context, &reducer, state.keep_open)),
								})
								.ok();
						},
						_ => (),
					}
				}

				// notify
				if let Some(notification) = notification {
					state.context.inner.application().dispatch(notification).ok();
				}
			},
			ReducerRequest::CreateStorage(id, result) => {
				// register
				match &result {
					Ok(storage) => {
						state.storages.insert(id.clone(), storage.clone());
					},
					Err(err) => {
						tracing::error!(co = ?id, ?err, "co-storage-failed");
					},
				}

				// respond pending
				let mut remove = state
					.pending_requests
					.iter()
					.enumerate()
					.filter_map(|(index, request)| match request {
						ReducerRequest::Storage(request_id, _) if request_id == &id => Some(index),
						_ => None,
					})
					.collect::<VecDeque<usize>>();
				while let Some(index) = remove.pop_back() {
					if let Some(ReducerRequest::Storage(_, response)) = state.pending_requests.remove(index) {
						// for the last element send the original result
						if remove.is_empty() {
							response.send(result).ok();
							break;
						} else {
							response
								.send(match &result {
									Err(err) => Err(co_reducerfactory_error_clone(err)),
									Ok(storage) => Ok(storage.clone()),
								})
								.ok();
						}
					}
				}
			},
		}
		return Ok(());
	}
}

fn co_reducer_instance(context: &CoContext, root_instance: &CoReducer, keep_open: bool) -> CoReducer {
	if keep_open {
		// return same instance
		root_instance.clone()
	} else {
		// return clean instance
		root_instance
			.clone_with_detached_storage()
			.with_overlay_storage(context.tasks(), context.inner.application_storage().clone())
	}
}

fn co_reducerfactory_error_clone(err: &CoReducerFactoryError) -> CoReducerFactoryError {
	match err {
		CoReducerFactoryError::CoNotFound(id, err) => {
			CoReducerFactoryError::CoNotFound(id.clone(), anyhow!("source: {:?}", err))
		},
		CoReducerFactoryError::Create(id, err) => {
			CoReducerFactoryError::Create(id.to_owned(), anyhow!("source: {:?}", err))
		},
		CoReducerFactoryError::Other(err) => CoReducerFactoryError::Other(anyhow!("source: {:?}", err)),
		CoReducerFactoryError::Actor(err) => CoReducerFactoryError::Other(anyhow!("source: {:?}", err)),
	}
}
