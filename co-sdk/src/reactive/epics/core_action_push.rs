use crate::{
	reactive::context::{ActionObservable, StateObservable},
	Action, CoContext, CoReducerFactory, ReducerChangeContext,
};
use anyhow::anyhow;
use co_identity::PrivateIdentityResolver;
use co_primitives::{CoId, ReducerAction};
use futures::{Stream, StreamExt};
use libipld::Ipld;

/// Apply `Action::CoreActionPush` to reducer.
pub fn core_action_push(
	actions: ActionObservable,
	_states: StateObservable,
	context: CoContext,
) -> impl Stream<Item = Action> + Send + 'static {
	actions
		.filter_map(|action| async move {
			match action {
				Action::CoreActionPush { co, action } => Some((co, action)),
				_ => None,
			}
		})
		.filter_map(move |(co, action)| {
			let context = context.clone();
			async move {
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
			}
		})
}

async fn push(context: &CoContext, co: &CoId, action: &ReducerAction<Ipld>) -> anyhow::Result<()> {
	let reducer = context.co_reducer(&co).await?.ok_or(anyhow!("Co not found: {}", co))?;
	let identity = context.private_identity_resolver().await?.resolve_private(&action.from).await?;
	reducer.push_action(&identity, action).await?;
	Ok(())
}
