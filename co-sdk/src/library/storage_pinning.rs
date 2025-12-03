use super::{
	memory_dispatch::MemoryDispatch,
	storage_cleanup::storage_cleanup,
	storage_structure::{storage_structure_recursive, CoStructureResolver},
};
use crate::{
	library::{
		create_reducer_action::create_reducer_action, storage_dispatch_remove::storage_dispatch_remove,
		storage_dispatch_roots::storage_dispatch_roots,
	},
	state::core_state,
	CoPinningKey, CoReducerState, DynamicCoDate, Runtime, Storage, CO_CORE_NAME_STORAGE, CO_ID_LOCAL,
};
use anyhow::anyhow;
use cid::Cid;
use co_actor::TaskSpawner;
use co_core_storage::{BlockInfo, StorageAction};
use co_identity::PrivateIdentityBox;
use co_log::EntryBlock;
use co_primitives::{
	BlockLinks, CoId, CoList, Link, OptionMappedCid, ReducerAction, StoreParams, WeakCoReferenceFilter,
};
use co_storage::{BlockStorage, BlockStorageContentMapping, BlockStorageExt, ExtendedBlockStorage, OverlayChange};
use futures::{pin_mut, stream, TryStreamExt};
use std::{collections::BTreeSet, time::Duration};

#[derive(Debug, Clone)]
pub struct StoragePinningContext {
	pub identity: PrivateIdentityBox,
	pub storage: Storage,
	pub runtime: Runtime,
	pub date: DynamicCoDate,
	pub tasks: TaskSpawner,
	pub block_links: BlockLinks,
	pub free: bool,
	pub verify_links: Option<BlockLinks>,
}

/// Apply pinning to storage core.
/// Return the next `local_state` if something has changed.
#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip_all)]
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
		context.verify_links.clone(),
	)
	.await?;
	let storage = dispatcher.storage().clone();

	// storage: remove
	//  note: we assume that removed block only belongs to state
	storage_dispatch_remove(
		&mut dispatcher,
		BlockInfo::new(local_storage, CoPinningKey::State.to_string(co_id), Default::default()).await?,
		stream::iter(co_removed_blocks),
		<S as BlockStorage>::StoreParams::MAX_BLOCK_SIZE,
	)
	.await?;

	// storage: pins
	storage_dispatch_roots(&storage, &mut dispatcher, &co_id, co_new_roots).await?;

	// caluculate and free?
	if context.free {
		let mut structure_filter =
			CoStructureResolver::new(co_id, context.block_links.clone().with_filter(WeakCoReferenceFilter::new()));

		// storage: references
		let state = dispatcher.state().into();
		storage_structure_recursive(&storage, &mut dispatcher, state, co_storage, max_duration, &mut structure_filter)
			.await?;

		// storage: cleanup
		let state = dispatcher.state().into();
		storage_cleanup(&storage, &mut dispatcher, state, co_storage, &mut structure_filter).await?;
	}

	// result
	let storage = dispatcher.storage().clone();
	let overlay_storage = dispatcher.overlay_storage().clone();
	let roots = dispatcher.take_new_roots();
	if let Some(state) = roots.last().and_then(|state| state.state()) {
		// create storage core state
		let storage_core_state = core_state(&storage, state.into(), CO_CORE_NAME_STORAGE.as_ref())
			.await?
			.ok_or(anyhow!("No storage core found: {:?}", state))?;

		// collapse actions into single batch action
		let mut actions = CoList::default().open(&storage).await?;
		for root in roots {
			for head in &root.1 {
				let block = storage.get(head).await?;
				let entry = EntryBlock::from_block(block)?;
				let action_reference: Link<ReducerAction<StorageAction>> = entry.entry().payload.into();
				let action = storage.get_value(&action_reference).await?;
				actions.push(action.payload).await?;
			}
		}
		let batch_action = StorageAction::Batch(actions.store().await?);
		let batch_reducer_action: Cid = create_reducer_action(
			&storage,
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
			.push_reference_with_core_state(batch_reducer_action.into(), storage_core_state, true)
			.await?;

		// flush
		let next_local_state = dispatcher.reducer_state();
		for cid in next_local_state.iter() {
			overlay_storage.flush(cid, Some(context.block_links.clone())).await?;
		}

		// flush changes
		let changes = overlay_storage.consume_changes();
		pin_mut!(changes);
		while let Some(change) = changes.try_next().await? {
			match change {
				OverlayChange::Remove(cid) => {
					// this will actually delte the blocks from storage_cleanup
					overlay_storage.next_storage().remove(&cid).await?;
				},
				_ => {},
			}
		}

		Ok(Some(next_local_state))
	} else {
		Ok(None)
	}
}
