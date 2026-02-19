// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

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
