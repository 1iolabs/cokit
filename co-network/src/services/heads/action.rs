// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_primitives::{CoId, NetworkCoHeads};
use std::collections::BTreeSet;

pub type Heads = BTreeSet<Cid>;

#[derive(Debug, Clone, derive_more::From)]
pub enum HeadsAction {
	Subscribe(SubscribeAction),
	Unsubscribe(UnsubscribeAction),
	Publish(PublishAction),
	Receive(ReceiveAction),
}

#[derive(Debug, Clone)]
pub struct SubscribeAction {
	pub network: NetworkCoHeads,
}

#[derive(Debug, Clone)]
pub struct UnsubscribeAction {
	pub network: NetworkCoHeads,
}

#[derive(Debug, Clone)]
pub struct PublishAction {
	pub network: NetworkCoHeads,
	pub heads: Heads,
}

#[derive(Debug, Clone)]
pub struct ReceiveAction {
	pub co: CoId,
	pub heads: Heads,
}
