use super::{
	memory_dispatch::MemoryDispatch, storage_cleanup::storage_cleanup, storage_structure::storage_structure_recursive,
};
use crate::{
	library::{
		create_reducer_action::create_reducer_action, storage_dispatch_remove::storage_dispatch_remove,
		storage_dispatch_roots::storage_dispatch_roots,
	},
	CoReducerState, DynamicCoDate, Runtime, Storage, CO_CORE_NAME_STORAGE, CO_ID_LOCAL,
};
use cid::Cid;
use co_actor::TaskSpawner;
use co_core_storage::StorageAction;
use co_identity::PrivateIdentityBox;
use co_log::EntryBlock;
use co_primitives::{BlockLinks, CoId, Link, OptionMappedCid, ReducerAction, StoreParams, WeakCoReferenceFilter};
use co_storage::{BlockStorage, BlockStorageContentMapping, BlockStorageExt, ExtendedBlockStorage};
use futures::stream;
use std::{collections::BTreeSet, time::Duration};

#[derive(Debug, Clone)]
pub struct StoragePinningContext {
	pub identity: PrivateIdentityBox,
	pub storage: Storage,
	pub runtime: Runtime,
	pub date: DynamicCoDate,
	pub tasks: TaskSpawner,
	pub block_links: BlockLinks,
}

/// Apply pinning to storage core.
/// Return the next `local_state` if something has changed.
#[tracing::instrument(err(Debug), skip_all)]
pub async fn storage_pinning<S>(
	context: &StoragePinningContext,
	max_duration: Option<Duration>,
	local_storage: &S,
	local_state: CoReducerState,
	co_id: &CoId,
	co_storage: &S,
	co_new_roots: Vec<CoReducerState>,
	co_removed_blocks: BTreeSet<OptionMappedCid>,
) -> Result<Option<CoReducerState>, anyhow::Error>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
{
	// create local memory reducer
	let mut dispatcher = MemoryDispatch::new(
		context.storage.clone(),
		context.runtime.clone(),
		context.date.clone(),
		context.tasks.clone(),
		CO_ID_LOCAL.into(),
		local_state.clone(),
		local_storage,
		context.identity.clone(),
		CO_CORE_NAME_STORAGE,
	)
	.await?;
	let storage = dispatcher.storage().clone();

	// storage: remove
	storage_dispatch_remove(
		&mut dispatcher,
		stream::iter(co_removed_blocks),
		<S as BlockStorage>::StoreParams::MAX_BLOCK_SIZE,
	)
	.await?;

	// storage: pins
	storage_dispatch_roots(&storage, &mut dispatcher, &co_id, co_new_roots).await?;

	// storage: references
	let state = dispatcher.state().into();
	storage_structure_recursive(
		&storage,
		&mut dispatcher,
		state,
		co_storage,
		context.block_links.clone().with_filter(WeakCoReferenceFilter::new()),
		max_duration,
	)
	.await?;

	// storage: cleanup
	let state = dispatcher.state().into();
	storage_cleanup(&mut dispatcher, &storage, state).await?;

	// result
	let overlay = dispatcher.storage().clone();
	let roots = dispatcher.take_new_roots();
	if let Some(state) = roots.last().and_then(|state| state.state()) {
		// collapse actions into single batch action
		let mut actions = Vec::new();
		for root in roots {
			for head in &root.1 {
				let block = overlay.get(head).await?;
				let entry = EntryBlock::from_block(block)?;
				let action_reference: Link<ReducerAction<StorageAction>> = entry.entry().payload.into();
				let action = overlay.get_value(&action_reference).await?;
				actions.push(overlay.set_value(&action.payload).await?);
			}
		}
		let batch_action = StorageAction::Batch(actions);
		let batch_reducer_action: Cid = create_reducer_action(
			&overlay,
			&context.identity,
			CO_CORE_NAME_STORAGE,
			batch_action,
			Default::default(),
			&context.date,
		)
		.await?
		.into();

		// apply
		dispatcher.reset(local_state).await?;
		dispatcher
			.push_reference_with_state(batch_reducer_action.into(), state, true)
			.await?;

		// flush
		let next_local_state = dispatcher.reducer_state();
		for cid in next_local_state.iter() {
			overlay.flush(cid, Some(context.block_links.clone())).await?;
		}
		Ok(Some(next_local_state))
	} else {
		Ok(None)
	}
}
