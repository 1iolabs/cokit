use crate::{
	library::network_identity::network_identity_by_id, services::application::action::ResolvePrivateIdentityAction,
	Action, ActionError, CoContext,
};
use co_actor::Actions;
use co_identity::PrivateIdentityResolver;
use futures::{FutureExt, Stream};

/// Resolve private identity.
pub fn resolve_private_identity(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::ResolvePrivateIdentity(action) => Some({
			let context = context.clone();
			let action = action.clone();
			async move {
				let result = resolve(&context, &action).await;
				Ok(Action::ResolvePrivateIdentityComplete(action, result))
			}
			.into_stream()
		}),
		_ => None,
	}
}

async fn resolve(
	context: &CoContext,
	action: &ResolvePrivateIdentityAction,
) -> Result<co_identity::PrivateIdentityBox, ActionError> {
	Ok(match action {
		ResolvePrivateIdentityAction::Identity { identity } => {
			let identity_resolver = context.private_identity_resolver().await?;
			identity_resolver.resolve_private(identity).await.map_err(anyhow::Error::from)?
		},
		ResolvePrivateIdentityAction::NetworkIdentity { parent_co, co } => {
			network_identity_by_id(context, parent_co, co, None).await?
		},
	})
}
