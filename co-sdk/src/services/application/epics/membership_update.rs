use crate::{Action, CoContext, CoReducerState, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL};
use co_actor::Actions;
use co_core_membership::MembershipsAction;
use co_storage::BlockStorageContentMapping;
use futures::{FutureExt, Stream, TryStreamExt};

/// When a membership is updated notify the reducer about it.
pub fn membership_update(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	// filter
	let result = match action {
		Action::CoreAction { co, storage, context: _, action, cid: _ }
			if co.as_str() == CO_ID_LOCAL && action.core == CO_CORE_NAME_MEMBERSHIP =>
		{
			let mambership_action: MembershipsAction = action.get_payload().ok()?;
			match mambership_action {
				MembershipsAction::Update { id, state, .. } => Some((storage.clone(), id, state)),
				_ => None,
			}
		},
		_ => None,
	};

	// update if state/heads are different
	let context = context.clone();
	Some(
		async move {
			if let Some((parent_storage, id, co_state)) = result {
				let control = context.inner.reducers_control();
				if let Some(reducer) = control.reducer_opt(id).await {
					let reducer_state = CoReducerState::from_co_state(&parent_storage, &co_state).await?;
					let next_reducer_state = reducer_state.to_internal(&parent_storage).await;
					let current_reducer_state = reducer.reducer_state().await;
					if current_reducer_state != next_reducer_state {
						// mappings
						if let Some(mappings) = next_reducer_state.to_external_mapping(&parent_storage).await {
							reducer.storage().insert_mappings(mappings).await;
						}

						// join
						reducer.join_state(next_reducer_state).await?;
					}
				}
			}
			Ok(())
		}
		.into_stream()
		.try_filter_map(|_| async { Ok(None) }),
	)
}

/// When a membership is removed clear the co_reducer instance.
pub fn membership_remove(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	// filter
	let result = match action {
		Action::CoreAction { co, storage: _, context: _, action, cid: _ }
			if co.as_str() == CO_ID_LOCAL && action.core == CO_CORE_NAME_MEMBERSHIP =>
		{
			let mambership_action: MembershipsAction = action.get_payload().ok()?;
			match mambership_action {
				MembershipsAction::Remove { id, did } => Some((id, did)),
				_ => None,
			}
		},
		_ => None,
	};

	// remove
	let context = context.clone();
	Some(
		async move {
			if let Some((id, _did)) = result {
				let control = context.inner.reducers_control();
				control.clear_one(id).await?;
			}
			Ok(())
		}
		.into_stream()
		.try_filter_map(|_| async { Ok(None) }),
	)
}
