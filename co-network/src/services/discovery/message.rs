// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::action::DiscoveryAction;
use crate::services::discovery;
use co_actor::ResponseStream;
use std::collections::BTreeSet;

#[derive(Debug)]
pub enum DiscoveryMessage {
	/// Connect peers using discovery.
	Connect(BTreeSet<discovery::Discovery>, ResponseStream<discovery::Event>),

	/// Internal action dispatch.
	Action(DiscoveryAction),
}
impl<T> From<T> for DiscoveryMessage
where
	T: Into<DiscoveryAction>,
{
	fn from(value: T) -> Self {
		Self::Action(value.into())
	}
}
