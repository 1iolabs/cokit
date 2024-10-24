use co_actor::Response;
use co_network::{Behaviour, Context, NetworkTaskBox};
use libp2p::PeerId;
use std::fmt::Debug;

#[derive(Debug)]
pub enum NetworkMessage {
	/// Spawn network task.
	Task(NetworkTaskBox<Behaviour, Context>),

	/// Get local PeerID.
	LocalPeerId(Response<PeerId>),
}
