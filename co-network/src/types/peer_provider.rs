use futures::Stream;
use libp2p::PeerId;
use std::collections::BTreeSet;

pub trait PeerProvider {
	/// Provide peers as a stream.
	/// Every time when new peers are discovered a item with all currently connected peers is emitted.
	/// The stream may completes when no more peers are discoverable.
	/// Errors will not neccesariliy end the stream.
	fn try_peers(&self) -> impl Stream<Item = Result<BTreeSet<PeerId>, anyhow::Error>> + Send + 'static;

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
