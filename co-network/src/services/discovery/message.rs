// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

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
