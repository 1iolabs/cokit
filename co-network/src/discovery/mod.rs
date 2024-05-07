use async_trait::async_trait;
use co_primitives::Network;
use libp2p::{swarm::NetworkBehaviour, PeerId};
use std::collections::BTreeSet;

pub mod did_discovery;
pub mod peer;

#[async_trait]
trait NetworkDiscovery<B>
where
	B: NetworkBehaviour,
{
	fn is_discoverable(&self, network: &Network) -> bool;
	async fn discover(&mut self, network: &Network) -> Result<BTreeSet<PeerId>, anyhow::Error>;
}
