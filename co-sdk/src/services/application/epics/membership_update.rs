use crate::{Action, CoContext, CoReducerState, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL};
use co_core_membership::MembershipsAction;
use futures::{FutureExt, Stream, TryStreamExt};

/// When a membership is updated notify the reducer about it.
pub fn membership_update(
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
				MembershipsAction::Update { id, state, heads, .. } => {
					Some((storage.clone(), id, CoReducerState::new_weak(Some(state), heads)))
				},
				_ => None,
			}
		},
		_ => None,
	};

	// update if state/heads are different
	let context = context.clone();
	Some(
		async move {
			if let Some((storage, id, reducer_state)) = result {
				let control = context.inner.reducers_control();
				if let Some(reducer) = control.reducer_opt(id).await {
					let current_reducer_state = reducer.reducer_state().await.to_external(&storage).await;
					if current_reducer_state != reducer_state {
						reducer.join_state(reducer_state).await?;
						// if let Some(parent_id) = reducer.parent_id() {
						// 	if let Some(parent) = control.reducer_opt(parent_id.clone()).await {
						// 		reducer.context.refresh(reducer.clone(), parent.clone()).await?;
						// 	}
						// }
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
