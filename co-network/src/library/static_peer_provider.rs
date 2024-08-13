use crate::PeerProvider;
use futures::{future::ready, prelude::Stream, stream};
use libp2p::PeerId;
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct StaticPeerProvider {
	peers: BTreeSet<PeerId>,
}
impl StaticPeerProvider {
	pub fn new(peers: BTreeSet<PeerId>) -> Self {
		Self { peers }
	}
}
impl PeerProvider for StaticPeerProvider {
	fn peers(&self) -> impl Stream<Item = BTreeSet<PeerId>> + Send + 'static {
		stream::once(ready(self.peers.clone()))
	}
}
