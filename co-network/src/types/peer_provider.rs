// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use futures::{Stream, StreamExt};
use libp2p::PeerId;
use std::{collections::BTreeSet, future::ready};

pub trait PeerProvider {
	/// Provide peers as a stream.
	/// Every time when new peers are discovered a item with all currently connected peers is emitted.
	/// The stream may completes when no more peers are discoverable.
	/// Errors will not neccesariliy end the stream.
	fn try_peers(&self) -> impl Stream<Item = Result<BTreeSet<PeerId>, anyhow::Error>> + Send + 'static;

	/// Provide peers as a stream.
	/// Every time when new peers are discovered a item with all currently connected peers is emitted.
	/// The stream may completes when no more peers are discoverable.
	/// This method is maybe lossy and is usually ignoring errors.
	fn peers(&self) -> impl Stream<Item = BTreeSet<PeerId>> + Send + 'static {
		self.try_peers().filter_map({
			move |result| {
				ready(match result {
					Ok(p) => Some(p),
					Err(err) => {
						tracing::warn!(?err, "peer-provider-error");
						None
					},
				})
			}
		})
	}

	/// Same as `peers` but only emit added peers.
	/// Initially all currently connected peers are returned.
	fn peers_added(&self) -> impl Stream<Item = BTreeSet<PeerId>> + Send + 'static {
		let peers_stream = self.peers();
		async_stream::stream! {
			let mut peers: BTreeSet<PeerId> = Default::default();
			for await next_peers in peers_stream {
				let added: BTreeSet<_> = next_peers.difference(&peers).collect();
				if !added.is_empty() {
					yield added.into_iter().cloned().collect();
				}
				peers = next_peers;
			}
		}
	}

	/// Same as `peers` but only emit added peers.
	/// Initially all currently connected peers are returned.
	fn try_peers_added(&self) -> impl Stream<Item = Result<BTreeSet<PeerId>, anyhow::Error>> + Send + 'static {
		let peers_stream = self.try_peers();
		async_stream::stream! {
			let mut peers: BTreeSet<PeerId> = Default::default();
			for await next_peers in peers_stream {
				match next_peers {
					Ok(next_peers) => {
						let added: BTreeSet<_> = next_peers.difference(&peers).collect();
						if !added.is_empty() {
							yield Ok(added.into_iter().cloned().collect());
						}
						peers = next_peers;
					},
					Err(err) => yield Err(err),
				}
			}
		}
	}
}
