use crate::{Action, CoContext};
use co_actor::Actions;
use co_identity::PeerDidCommHeader;
use co_network::services::connections::{ConnectionAction, PeerRelateDidAction};
use co_primitives::CoTryStreamExt;
use futures::{FutureExt, Stream};
use libp2p::PeerId;
use std::{str::FromStr, time::Instant};

/// When receive a DidComm message with an verified peer header relate it with the Did for future connections.
///
/// In: [`Action::DidCommReceive`]
pub fn didcomm_connected(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::DidCommReceive { peer: _, message } => Some({
			let context = context.clone();
			let from = message.sender().map(ToOwned::to_owned);
			let header = message.header().clone();
			async move {
				if let Some(from) = from {
					let header = PeerDidCommHeader::from(header);
					if let Some(peer) = &header.from_peer_id {
						if let Some(network) = context.network_connections().await {
							network.dispatch(ConnectionAction::PeerRelateDid(PeerRelateDidAction {
								did: from.to_owned(),
								peer_id: PeerId::from_str(peer)?,
								time: Instant::now(),
							}))?;
						}
					}
				}
				Ok(())
			}
			.into_stream()
			.try_ignore_elements()
		}),
		_ => None,
	}
}
