// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

pub mod connections;
pub mod dial;
pub mod didcomm_receive;
pub mod didcomm_send;
pub mod discovery;
pub mod gossip;
pub mod identify_dial;
pub mod listeners;
#[cfg(feature = "native")]
pub mod mdns_gossip;
pub mod peers;
pub mod relay_listen;
