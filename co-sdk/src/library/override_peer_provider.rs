use co_network::PeerProvider;
use co_primitives::CoId;
use futures::Stream;
use libp2p::PeerId;
use std::{
	collections::{BTreeMap, BTreeSet},
	sync::Arc,
};
use tokio::sync::RwLock;

/// Return explicitly overriden peers.
#[derive(Debug, Clone, Default)]
pub struct Overrides {
	peers: Arc<RwLock<BTreeMap<CoId, BTreeSet<PeerId>>>>,
}
impl Overrides {
	pub async fn set(&self, co: CoId, peers: BTreeSet<PeerId>) {
		self.peers.write().await.insert(co, peers);
	}

	pub async fn remove(&self, co: &CoId) {
		self.peers.write().await.remove(co);
	}

	pub async fn get(&self, co: &CoId) -> Option<BTreeSet<PeerId>> {
		self.peers.read().await.get(co).cloned()
	}
}

pub struct OverridePeerProvider<P> {
	overrides: Overrides,
	next: P,
	id: CoId,
}
impl<P> OverridePeerProvider<P> {
	pub fn new(overrides: Overrides, next: P, id: CoId) -> Self {
		Self { overrides, next, id }
	}
}
impl<P> PeerProvider for OverridePeerProvider<P>
where
	P: PeerProvider + Send + Sync + 'static,
{
	fn try_peers(&self) -> impl Stream<Item = Result<BTreeSet<PeerId>, anyhow::Error>> + Send + 'static {
		let id = self.id.clone();
		let overrides = self.overrides.clone();
		let next = self.next.try_peers();
		async_stream::stream! {
			if let Some(peers) = overrides.get(&id).await {
				yield Ok(peers);
			} else {
				for await peers in next {
					yield peers;
				}
			}
		}
	}

	fn peers(&self) -> impl Stream<Item = BTreeSet<PeerId>> + Send + 'static {
		let id = self.id.clone();
		let overrides = self.overrides.clone();
		let next = self.next.peers();
		async_stream::stream! {
			if let Some(peers) = overrides.get(&id).await {
				yield peers;
			} else {
				for await peers in next {
					yield peers;
				}
			}
		}
	}
}
