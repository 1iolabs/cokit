// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
