use crate::{
	services::{
		heads::{
			actor::{to_topic_hash, HeadsContext, HeadsState},
			HeadsAction, ReceiveAction,
		},
		network::ListenGossipTask,
	},
	HeadsMessage,
};
use co_actor::Actions;
use co_primitives::{from_cbor, WeakCid};
use futures::{Stream, StreamExt, TryStreamExt};

/// Listen to GossipSub messages and handle ours.
pub fn listen(
	_actions: &Actions<HeadsAction, HeadsState, HeadsContext>,
	action: &HeadsAction,
	state: &HeadsState,
	context: &HeadsContext,
) -> Option<impl Stream<Item = Result<HeadsAction, anyhow::Error>> + Send + 'static> {
	match action {
		HeadsAction::Subscribe(action) => {
			let action = action.clone();
			let topic_hash = to_topic_hash(&action.network);
			let is_first_subscribe = state
				.heads
				.get(&topic_hash)
				.map(|subscriptions| subscriptions.len() == 1)
				.unwrap_or(false);
			if is_first_subscribe {
				Some(
					ListenGossipTask::subscribe(context.network.clone(), topic_hash)
						.map(Ok)
						.try_filter_map(|gossip_message| async move {
							let heads_message: HeadsMessage =
								from_cbor(gossip_message.data()).map_err(anyhow::Error::from)?;
							match heads_message {
								HeadsMessage::Heads(co, heads) => Ok(Some(HeadsAction::Receive(ReceiveAction {
									co,
									heads: heads.iter().map(WeakCid::cid).collect(),
								}))),
								_ => Ok(None),
							}
						}),
				)
			} else {
				None
			}
		},
		_ => None,
	}
}
