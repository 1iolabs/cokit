// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

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

	fn try_peers(&self) -> impl Stream<Item = Result<BTreeSet<PeerId>, anyhow::Error>> + Send + 'static {
		stream::once(ready(Ok(self.peers.clone())))
	}
}
