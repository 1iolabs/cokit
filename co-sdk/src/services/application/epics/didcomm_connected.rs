// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{Action, CoContext};
use co_actor::{time, Actions};
use co_identity::PeerDidCommHeader;
use co_network::{
	connections::{ConnectionAction, PeerRelateDidAction},
	PeerId,
};
use co_primitives::CoTryStreamExt;
use futures::{FutureExt, Stream};
use std::str::FromStr;

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
								time: time::Instant::now(),
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
