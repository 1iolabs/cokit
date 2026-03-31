// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::co_heads_publish::network_co_heads;
use crate::{types::co_reducer_context::CoReducerFeature, Action, CoContext, CoReducerFactory};
use co_actor::{Actions, Epic};
use co_primitives::{CoId, NetworkCoHeads};
use futures::Stream;
use std::collections::BTreeSet;

/// Whe open/close a co subscribe to heads.
#[derive(Debug, Clone, Default)]
pub struct CoHeadsSubscribeEpic {}
impl Epic<Action, (), CoContext> for CoHeadsSubscribeEpic {
	fn epic(
		&mut self,
		_actions: &Actions<Action, (), CoContext>,
		action: &Action,
		_state: &(),
		context: &CoContext,
	) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
		let (co, mode) = match action {
			Action::CoOpen { co, network: true } => (co.clone(), true),
			Action::CoClose { co } => (co.clone(), false),
			_ => {
				return None;
			},
		};
		Some(Action::future_ignore_elements(subscribe(context.clone(), co, mode)))
	}
}

async fn subscribe(context: CoContext, co: CoId, mode: bool) -> Result<(), anyhow::Error> {
	// network
	let Some(heads) = context.network_heads().await else {
		return Ok(());
	};

	// get current networks
	//  when mode is set to false we want to unsubscribe all from this co
	let networks: BTreeSet<NetworkCoHeads> = if mode {
		// get co
		let co_reducer = context.try_co_reducer(&co).await?;
		if !co_reducer.context.has_feature(&CoReducerFeature::Network) {
			return Ok(());
		}
		let co_reducer_state = co_reducer.reducer_state().await;
		let storage = co_reducer.storage();

		// get networks
		network_co_heads(&storage, co.clone(), co_reducer_state.co()).await?.collect()
	} else {
		Default::default()
	};

	// subscribe
	for network in networks.into_iter() {
		if mode {
			heads.subscribe(network)?
		} else {
			heads.unsubscribe(network)?
		}
	}

	// result
	Ok(())
}
