// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	library::{local_keypair_fetch::local_keypair_fetch, network_resolver::CoNetworkResolver},
	services::bitswap::Bitswap,
	Action, ActionError, CoContext,
};
use co_actor::{Actions, Actor};
use co_network::{connections::DynamicNetworkResolver, Network, NetworkInitialize, NetworkMessage, NetworkSettings};
use co_primitives::tags;
use futures::{FutureExt, Stream};

pub fn network_start(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::NetworkStart(settings) => {
			let context = context.clone();
			let settings = settings.clone();
			Some(
				async move {
					Ok(Action::NetworkStartComplete(start(context, settings).await.map_err(ActionError::from)))
				}
				.into_stream(),
			)
		},
		_ => None,
	}
}

async fn start(context: CoContext, settings: NetworkSettings) -> Result<(), anyhow::Error> {
	// resolve key
	let local_identity = context.local_identity();
	let local_co = context.local_co_reducer().await?;
	let network_key =
		local_keypair_fetch(context.identifier(), &local_co, &local_identity, settings.force_new_peer_id).await?;

	// bitswap
	let bitswap = Actor::spawn_with(
		context.tasks(),
		tags!("type": "bitswap", "application": context.identifier()),
		Bitswap::new(context.clone()),
		(),
	)?;

	// network
	let network_initialize = NetworkInitialize {
		bitswap: bitswap.handle(),
		identifier: context.identifier().to_owned(),
		identity_resolver: context.identity_resolver().await?,
		keypair: network_key,
		private_identity_resolver: context.private_identity_resolver().await?,
		tasks: context.tasks(),
		settings,
		network_resolver: DynamicNetworkResolver::new(CoNetworkResolver::new(context.clone())),
	};
	let network = Actor::spawn_with(
		context.tasks(),
		tags!("type": "network", "application": context.identifier()),
		Network,
		network_initialize,
	)?;

	// initialize
	network.handle().initialized().await?;

	// set network to reducers
	let network = network.handle().request(NetworkMessage::Network).await?;
	context.inner.set_network(Some(network)).await?;

	// done
	Ok(())
}
