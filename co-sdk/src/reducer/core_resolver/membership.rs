use crate::{CoreResolver, CoreResolverError, ReducerChangeContext, TaskSpawner};
use anyhow::Context;
use async_trait::async_trait;
use co_primitives::{BlockSerializer, CoId, ReducerAction};
use co_runtime::RuntimePool;
use co_storage::BlockStorage;
use libipld::{store::StoreParams, Block, Cid};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Update instance when membership changes.
/// This is implemented as [`CoreResolver`] middleware because we can just check if the action is relevant.
pub struct MembershipCoreResolver<S, C, R> {
	tasks: TaskSpawner,
	membership_core_name: String,
	registry: R,
	next: C,
	_storage: PhantomData<S>,
}
impl<S, C, R> MembershipCoreResolver<S, C, R>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
	C: CoreResolver<S> + Send + Sync + 'static,
	R: MembershipInstanceRegistry + Clone + Send + Sync + 'static,
{
	pub fn new(tasks: TaskSpawner, next: C, registry: R, membership_core_name: String) -> Self {
		Self { tasks, membership_core_name, registry, next, _storage: Default::default() }
	}

	async fn try_update_membership(
		registry: R,
		action_membership: MinimalMembershipsAction,
	) -> Result<(), anyhow::Error> {
		match action_membership {
			MinimalMembershipsAction::Update { id } => registry.update(id).await,
			MinimalMembershipsAction::Remove { id } => registry.remove(id).await,
		}
	}
}
#[async_trait]
impl<S, C, R> CoreResolver<S> for MembershipCoreResolver<S, C, R>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
	C: CoreResolver<S> + Send + Sync + 'static,
	R: MembershipInstanceRegistry + Clone + Send + Sync + 'static,
{
	async fn execute(
		&self,
		storage: &S,
		runtime: &RuntimePool,
		context: &ReducerChangeContext,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<Option<Cid>, CoreResolverError> {
		// execute
		let next_state = self.next.execute(storage, runtime, context, state, action).await?;

		// membership
		if !context.is_local_change() {
			let action_block = storage
				.get(action)
				.await
				.map_err(|e| CoreResolverError::InvalidArgument(e.into()))
				.context("resolving action")?;
			if let Some(action_membership) = MinimalMembershipsAction::new(&action_block, &self.membership_core_name) {
				let registry = self.registry.clone();
				self.tasks.spawn(async move {
					if let Err(err) = Self::try_update_membership(registry, action_membership).await {
						tracing::warn!(?err, "membership-update-failed");
					}
				});
			}
		}

		// result
		Ok(next_state)
	}
}

/// Light clone of [`co_core_membership::MembershipsAction`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum MinimalMembershipsAction {
	Update { id: CoId },
	Remove { id: CoId },
}
impl MinimalMembershipsAction {
	fn new<S: StoreParams>(block: &Block<S>, membership_core_name: &str) -> Option<Self> {
		let action = BlockSerializer::new().deserialize::<ReducerAction<MinimalMembershipsAction>>(&block);
		if let Ok(action) = action {
			if action.core == membership_core_name {
				return Some(action.payload);
			}
		}
		None
	}
}

#[async_trait]
pub trait MembershipInstanceRegistry {
	async fn update(&self, co: CoId) -> Result<(), anyhow::Error>;
	async fn remove(&self, co: CoId) -> Result<(), anyhow::Error>;
}
