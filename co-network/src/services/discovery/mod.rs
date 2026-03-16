// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

pub mod action;
pub mod actor;
mod api;
mod epics;
mod event;
pub mod message;
pub mod state;
mod types;

pub use actor::{DiscoveryActor, DiscoveryContext};
pub use api::DiscoveryApi;
pub use event::Event;
pub use message::DiscoveryMessage;
pub use types::{DidDiscovery, DidDiscoveryMessageType, DiscoverMessage, Discovery};
