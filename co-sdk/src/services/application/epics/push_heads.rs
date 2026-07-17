// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	library::head_delivery::push_heads_to_dids, services::reducer::FlushInfo, state, Action, CoContext,
	CoReducerFactory, HeadsRecipient,
};
use co_actor::{Actions, Epic};
use co_primitives::Did;
use futures::Stream;

#[derive(Debug, Default)]
pub struct PushHeadsEpic;

impl Epic<Action, (), CoContext> for PushHeadsEpic {
	fn epic(
		&mut self,
		_actions: &Actions<Action, (), CoContext>,
		action: &Action,
		_state: &(),
		context: &CoContext,
	) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
		let (co, from) = match action {
			Action::CoFlush { co, info: FlushInfo { local: true, network: true, local_identity: Some(from) } } => {
				(co.clone(), from.clone())
			},
			_ => return None,
		};
		Some(Action::future_ignore_elements(push_participant_heads(context.clone(), co, from)))
	}
}

async fn push_participant_heads(context: CoContext, co: co_primitives::CoId, from: Did) -> Result<(), anyhow::Error> {
	let reducer = context.try_co_reducer(&co).await?;
	let state = reducer.reducer_state().await;
	let recipients = automatic_participant_recipients(
		&from,
		state::participants_active(&reducer.storage(), state.co())
			.await?
			.into_iter()
			.map(|participant| participant.did),
	);
	push_heads_to_dids(&context, co, from, recipients)?;
	Ok(())
}

fn automatic_participant_recipients(from: &Did, participants: impl IntoIterator<Item = Did>) -> Vec<HeadsRecipient> {
	participants
		.into_iter()
		.filter(|did| did != from)
		.map(|did| HeadsRecipient { did, connectivity: Default::default() })
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn automatic_participant_recipients_exclude_sender() {
		let from = Did::from("did:key:sender");
		let remote = Did::from("did:key:remote");

		let recipients = automatic_participant_recipients(&from, [from.clone(), remote.clone()]);

		assert_eq!(recipients.into_iter().map(|recipient| recipient.did).collect::<Vec<_>>(), [remote]);
	}
}
