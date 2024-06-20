use futures::Stream;
use libp2p::PeerId;
use std::collections::BTreeSet;

pub trait PeerProvider {
	/// Provide peers as a stream.
	/// Every time when new peers are discovered a item with all currently connected peers is emitted.
	/// The stream may completes when no more peers are discoverable.
	fn peers(&self) -> impl Stream<Item = BTreeSet<PeerId>> + Send + 'static;

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
}
