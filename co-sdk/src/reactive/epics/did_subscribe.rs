use crate::{
	drivers::network::{
		subscribe::{subscribe_identity, unsubscribe_identity},
		CoNetworkTaskSpawner,
	},
	reactive::context::{ActionObservable, StateObservable},
	state::{self, core_state_or_default},
	Action, CoContext, CoStorage, CO_CORE_NAME_KEYSTORE, CO_ID_LOCAL,
};
use co_core_co::Co;
use co_core_keystore::{Key, KeyStore, KeyStoreAction};
use co_identity::{PrivateIdentityResolver, PrivateIdentityResolverBox};
use co_primitives::{Did, OptionLink};
use futures::{pin_mut, Stream, StreamExt};

/// Subscribe DIDs when network is started.
pub fn network_started(
	actions: ActionObservable,
	_states: StateObservable,
	context: CoContext,
) -> impl Stream<Item = Action> + Send + 'static {
	actions
		.filter_map(|action| async move {
			match action {
				Action::NetworkStarted => Some(()),
				_ => None,
			}
		})
		.filter_map({
			let context = context.clone();
			move |_| {
				let context = context.clone();
				async move { context.network().await }
			}
		})
		.flat_map(move |network| subscribe_all(context.clone(), network).map(Action::map_error))
}

/// Subscribe/Unsubscribe DID when it gets created/removed.
pub fn keystore_changed(
	actions: ActionObservable,
	_states: StateObservable,
	context: CoContext,
) -> impl Stream<Item = Action> + Send + 'static {
	apply_subscribe_actions(
		context.clone(),
		actions.filter_map(move |action| {
			let context = context.clone();
			async move {
				match action {
					Action::CoreAction { co, context: change_context, action, cid: _ }
						if co.as_str() == CO_ID_LOCAL
							&& change_context.is_local_change()
							&& action.core == CO_CORE_NAME_KEYSTORE =>
					{
						let keystore_action: KeyStoreAction = action.get_payload().ok()?;
						match keystore_action {
							KeyStoreAction::Set(key) if state::is_identity(&key) => {
								Some(SubscribeAction::Subscribe(key.uri))
							},
							KeyStoreAction::Remove(remove_uri) => {
								let local_co = context.local_co_reducer().await.ok()?;
								let remove_key =
									key_by_uri(&local_co.storage(), local_co.co_state().await, &remove_uri)
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
					},
					_ => None,
				}
			}
		}),
	)
	.map(Action::map_error)
}

enum SubscribeAction {
	Subscribe(Did),
	Unsubscribe(Did),
}

fn apply_subscribe_actions(
	context: CoContext,
	actions: impl Stream<Item = SubscribeAction>,
) -> impl Stream<Item = Result<Action, anyhow::Error>> {
	async_stream::try_stream! {
		let private_identity_resolver = context.private_identity_resolver().await?;
		for await action in actions {
			if let Some(network) = context.network().await {
				let result = match action {
					SubscribeAction::Subscribe(did) => {
						subscribe(&private_identity_resolver, &network, &did).await
					},
					SubscribeAction::Unsubscribe(did) => {
						unsubscribe_identity(&network, did).await
					},
				};
				match result {
					Ok(()) => {},
					Err(err) => {
						yield Action::from(err);
					}
				}
			}
		}
	}
}

async fn key_by_uri(storage: &CoStorage, co: OptionLink<Co>, uri: &str) -> Result<Option<Key>, anyhow::Error> {
	let keystore: KeyStore = core_state_or_default(storage, co, CO_CORE_NAME_KEYSTORE).await?;
	let keys = state::stream(storage.clone(), &keystore.keys);
	pin_mut!(keys);
	let mut first_error: Option<anyhow::Error> = None;
	while let Some(key) = keys.next().await {
		match key {
			Ok((key_uri, key)) => {
				if &key_uri == uri {
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

fn subscribe_all(
	context: CoContext,
	network: CoNetworkTaskSpawner,
) -> impl Stream<Item = Result<Action, anyhow::Error>> {
	async_stream::try_stream! {
		let local_co = context.local_co_reducer().await?;
		let private_identity_resolver = context.private_identity_resolver().await?;
		for await identity in state::identities(local_co.storage(), local_co.co_state().await, None) {
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
	network: &CoNetworkTaskSpawner,
	did: &Did,
) -> Result<(), anyhow::Error> {
	let identity = private_identity_resolver.resolve_private(&did).await?;
	subscribe_identity(network, &identity).await?;
	Ok(())
}
