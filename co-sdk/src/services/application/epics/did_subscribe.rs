// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	state::{self, query_core, Query},
	Action, CoContext, CoStorage, CO_CORE_NAME_KEYSTORE, CO_ID_LOCAL,
};
use co_actor::Actions;
use co_core_co::Co;
use co_core_keystore::{Key, KeyStoreAction};
use co_identity::{PrivateIdentityResolver, PrivateIdentityResolverBox};
use co_network::{subscribe_identity, NetworkApi};
use co_primitives::{CoTryStreamExt, Did, OptionLink};
use futures::{future::Either, pin_mut, stream, FutureExt, Stream, StreamExt};

/// Subscribe DIDs when network is started.
pub fn network_started(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::NetworkStartComplete(Ok(())) => Some({
			let context = context.clone();
			async move {
				if let Some(network) = context.network().await {
					Either::Left(subscribe_all(context.clone(), network))
				} else {
					Either::Right(stream::empty())
				}
			}
			.into_stream()
			.flatten()
		}),
		_ => None,
	}
}

/// Subscribe/Unsubscribe DID when it gets created/removed.
pub fn keystore_changed(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::CoreAction { co, context: change_context, action, cid: _, storage: _, head: _ }
			if co.as_str() == CO_ID_LOCAL
				&& change_context.is_local_change()
				&& CO_CORE_NAME_KEYSTORE == action.core =>
		{
			if let Ok(keystore_action) = action.get_payload::<KeyStoreAction>() {
				Some({
					let context = context.clone();
					async move {
						if let Some(subscribe_action) =
							SubscribeAction::from_keystore_action(&context, keystore_action).await
						{
							if let Some(network) = context.network().await {
								let private_identity_resolver = context.private_identity_resolver().await?;
								match subscribe_action {
									SubscribeAction::Subscribe(did) => {
										subscribe(&private_identity_resolver, &network, &did).await?;
									},
									SubscribeAction::Unsubscribe(did) => {
										network.didcontact_unsubscribe(did).await?;
									},
								}
							}
						}
						Ok(())
					}
					.into_stream()
					.try_ignore_elements()
				})
			} else {
				None
			}
		},
		_ => None,
	}
}

enum SubscribeAction {
	Subscribe(Did),
	Unsubscribe(Did),
}
impl SubscribeAction {
	async fn from_keystore_action(context: &CoContext, keystore_action: KeyStoreAction) -> Option<SubscribeAction> {
		match keystore_action {
			KeyStoreAction::Set(key) if state::is_identity(&key) => Some(SubscribeAction::Subscribe(key.uri)),
			KeyStoreAction::Remove(remove_uri) => {
				let local_co = context.local_co_reducer().await.ok()?;
				let remove_key = key_by_uri(&local_co.storage(), local_co.reducer_state().await.co(), &remove_uri)
					.await
					.ok()??;
				if state::is_identity(&remove_key) {
					Some(SubscribeAction::Unsubscribe(remove_uri))
				} else {
					None
				}
			},
			_ => None,
		}
	}
}

async fn key_by_uri(storage: &CoStorage, co: OptionLink<Co>, uri: &str) -> Result<Option<Key>, anyhow::Error> {
	let keystore = query_core(CO_CORE_NAME_KEYSTORE).with_default().execute(storage, co).await?;
	let keys = state::stream(storage.clone(), &keystore.keys);
	pin_mut!(keys);
	let mut first_error: Option<anyhow::Error> = None;
	while let Some(key) = keys.next().await {
		match key {
			Ok((key_uri, key)) => {
				if key_uri == uri {
					return Ok(Some(key));
				}
			},
			Err(err) => {
				if first_error.is_none() {
					first_error = Some(err.into());
				}
			},
		}
	}
	first_error.map(Err).unwrap_or(Ok(None))
}

fn subscribe_all(context: CoContext, network: NetworkApi) -> impl Stream<Item = Result<Action, anyhow::Error>> {
	async_stream::try_stream! {
		let local_co = context.local_co_reducer().await?;
		let private_identity_resolver = context.private_identity_resolver().await?;
		for await identity in state::identities(local_co.storage(), local_co.reducer_state().await.co(), None) {
			let identity = match identity {
				Ok(i) => i,
				Err(err) => {
					yield Action::from(Into::<anyhow::Error>::into(err));
					continue;
				},
			};
			match subscribe(&private_identity_resolver, &network, &identity.did).await {
				Ok(()) => {},
				Err(err) => yield Action::from(err),
			}
		}
	}
}

async fn subscribe(
	private_identity_resolver: &PrivateIdentityResolverBox,
	network: &NetworkApi,
	did: &Did,
) -> Result<(), anyhow::Error> {
	let identity = private_identity_resolver.resolve_private(did).await?;
	subscribe_identity(network, &identity).await?;
	Ok(())
}
