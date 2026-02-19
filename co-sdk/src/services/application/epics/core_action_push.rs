// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{Action, CoContext, CoReducerFactory, ReducerChangeContext};
use co_actor::Actions;
use co_identity::PrivateIdentityResolver;
use co_primitives::{CoId, ReducerAction};
use futures::{stream, Stream, StreamExt};
use ipld_core::ipld::Ipld;

/// Apply `Action::CoreActionPush` to reducer.
pub fn core_action_push(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::CoreActionPush { co, action } => Some(
			stream::iter([(context.clone(), co.clone(), action.clone())])
				.filter_map(move |(context, co, action)| async move {
					if let Err(err) = push(&context, &co, &action).await {
						Some(Action::CoreActionFailure {
							co,
							context: ReducerChangeContext::new(),
							action,
							err: err.into(),
						})
					} else {
						None
					}
				})
				.map(Ok),
		),
		_ => None,
	}
}

async fn push(context: &CoContext, co: &CoId, action: &ReducerAction<Ipld>) -> anyhow::Result<()> {
	let reducer = context.try_co_reducer(co).await?;
	let identity = context.private_identity_resolver().await?.resolve_private(&action.from).await?;
	reducer.push_action(&identity, action).await?;
	Ok(())
}
