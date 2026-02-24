// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{Action, CoContext};
use co_actor::Actions;
use futures::{future::ready, stream, Stream, StreamExt};

/// Receive DIDComm messages after the network has been started.
pub fn didcomm_receive(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::NetworkStartComplete(Ok(())) => Some({
			stream::once(ready(context.clone()))
				.filter_map(|context| async move { context.network().await })
				.flat_map(|network| network.didcomm_receive())
				.map(|(peer, message)| Action::DidCommReceive { peer, message })
				.map(Ok)
		}),
		_ => None,
	}
}
