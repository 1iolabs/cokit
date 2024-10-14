use crate::{
	drivers::network::tasks::didcomm_receive::DidCommReceiveNetworkTask,
	reactive::context::{ActionObservable, StateObservable},
	Action, CoContext,
};
use futures::{future::ready, Stream, StreamExt};

/// Receive DIDComm messages after the network has been started.
pub fn didcomm_receive(
	actions: ActionObservable,
	_states: StateObservable,
	context: CoContext,
) -> impl Stream<Item = Action> + Send + 'static {
	actions
		.filter(|action| ready(matches!(action, Action::NetworkStarted)))
		.filter_map(move |_| {
			let context = context.clone();
			async move { context.network_tasks().await }
		})
		.flat_map(|network| DidCommReceiveNetworkTask::receive(network))
		.map(|(peer, message)| Action::DidCommReceive { peer, message })
}
