// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

pub mod action;
mod actor;
mod epics;
mod library;
mod message;
mod resolve;
mod state;

pub use actor::{Connections, ConnectionsContext};
pub use message::ConnectionMessage;
pub use resolve::{DynamicNetworkResolver, NetworkResolver};
pub use state::{CoConnection, ConnectionState, NetworkConnection, PeerConnection};
