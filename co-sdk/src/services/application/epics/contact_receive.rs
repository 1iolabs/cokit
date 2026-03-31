// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	library::{contact::CO_DIDCOMM_CONTACT, contact_handler::ContactHandler},
	Action, CoContext,
};
use co_actor::Actions;
use futures::Stream;

/// Handle incoming contact requests via the configured handler.
///
/// In: [`Action::DidCommReceive`] with message type `co-contact`
pub fn contact_receive(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::DidCommReceive { peer: _, message } => {
			if message.header().message_type != CO_DIDCOMM_CONTACT {
				return None;
			}
			let sender = message.sender().cloned()?;
			let handler = context.contact_handler()?.clone();
			let (header, _body) = message.clone().into_inner();
			Some(Action::future_ignore_elements(async move { handler.handle_contact(&sender, &header).await }))
		},
		_ => None,
	}
}
