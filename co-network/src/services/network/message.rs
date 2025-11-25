use crate::services::network::NetworkApi;
use co_actor::Response;
use libp2p::PeerId;
use std::fmt::Debug;

#[derive(Debug)]
pub enum NetworkMessage {
	/// Get local PeerID.
	LocalPeerId(Response<PeerId>),

	/// Get network APIs.
	Network(Response<NetworkApi>),
}
