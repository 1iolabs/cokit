use crate::{drivers::network::tasks::didcomm_receive::DidCommReceiveNetworkTask, Action, CoContext};
use futures::{future::ready, stream, Stream, StreamExt};

/// Receive DIDComm messages after the network has been started.
pub fn didcomm_receive(
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::NetworkStarted => Some({
			stream::once(ready(context.clone()))
				.filter_map(|context| async move { context.network_tasks().await })
				.flat_map(|network| DidCommReceiveNetworkTask::receive(network))
				.map(|(peer, message)| Action::DidCommReceive { peer, message })
				.map(Ok)
		}),
		_ => None,
	}
}
