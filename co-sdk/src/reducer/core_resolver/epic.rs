use crate::{services::application::ApplicationMessage, Action, CoreResolver, CoreResolverError, ReducerChangeContext};
use async_trait::async_trait;
use cid::Cid;
use co_actor::ActorHandle;
use co_primitives::CoId;
use co_runtime::{RuntimeContext, RuntimePool};
use co_storage::BlockStorage;
use std::marker::PhantomData;

/// Epic resolver middleware.
#[derive(Debug, Clone)]
pub struct ReactiveCoreResolver<S, N> {
	_storage: PhantomData<S>,
	next: N,
	co: CoId,
	actions: ActorHandle<ApplicationMessage>,
}
impl<S, N> ReactiveCoreResolver<S, N>
where
	S: BlockStorage + Clone + Send + Sync + 'static,
	N: CoreResolver<S> + Clone + Send + Sync + 'static,
{
	pub fn new(next: N, co: CoId, actions: ActorHandle<ApplicationMessage>) -> Self {
		Self { _storage: Default::default(), co, next, actions }
	}
}
#[async_trait]
impl<S, N> CoreResolver<S> for ReactiveCoreResolver<S, N>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
	N: CoreResolver<S> + Send + Sync + 'static,
{
	async fn execute(
		&self,
		storage: &S,
		runtime: &RuntimePool,
		context: &ReducerChangeContext,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<RuntimeContext, CoreResolverError> {
		// execute
		let next_state = self.next.execute(storage, runtime, context, state, action).await?;

		// dispatch
		// self.states.dispatch((
		// 	self.co.clone(),
		// 	next_state
		// 		.ok_or_else(|| CoreResolverError::Middleware(anyhow!("Expected a state after execute the action")))?
		// 		.into(),
		// ));
		self.actions
			.dispatch(
				Action::core_action(storage, self.co.clone(), context.clone(), action.into())
					.await
					.map_err(|err| CoreResolverError::Middleware(err.into()))?,
			)
			.map_err(|err| CoreResolverError::Middleware(err.into()))?;

		// result
		Ok(next_state)
	}
}
