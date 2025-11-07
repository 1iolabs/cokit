use crate::{
	network::{Behaviour, Context},
	services::network::NetworkApi,
	NetworkTaskBox,
};
use co_actor::Response;
use libp2p::PeerId;
use std::fmt::Debug;

#[derive(Debug)]
pub enum NetworkMessage {
	/// Spawn network task.
	Task(NetworkTaskBox<Behaviour, Context>),

	/// Get local PeerID.
	LocalPeerId(Response<PeerId>),

	/// Get network APIs.
	Network(Response<NetworkApi>),
}
