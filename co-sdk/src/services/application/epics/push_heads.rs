// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	library::push_heads::PushHeads, services::reducer::FlushInfo, types::co_reducer_context::CoReducerFeature, Action,
	CoContext, CoReducerFactory,
};
use co_actor::{Actions, Epic};
use co_identity::PrivateIdentityResolver;
use co_primitives::{CoId, Did};
use futures::Stream;
use std::{
	collections::HashMap,
	sync::{Arc, Mutex},
};

/// TODO: free instances (when close reducer?)
#[derive(Debug, Default, Clone)]
pub struct PushHeadsEpic {
	instances: Arc<Mutex<HashMap<CoId, PushHeads>>>,
}
impl Epic<Action, (), CoContext> for PushHeadsEpic {
	fn epic(
		&mut self,
		_actions: &Actions<Action, (), CoContext>,
		action: &Action,
		_state: &(),
		context: &CoContext,
	) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
		let (co, identity) = match action {
			Action::CoFlush { co, info: FlushInfo { local: true, network: true, local_identity: Some(identity) } } => {
				(co.clone(), identity.clone())
			},
			_ => {
				return None;
			},
		};
		Some(Action::future_ignore_elements(push(context.clone(), self.clone(), co, identity)))
	}
}

async fn push(context: CoContext, epic: PushHeadsEpic, co: CoId, identity: Did) -> Result<(), anyhow::Error> {
	// network
	let Some(network) = context.network().await else {
		return Ok(());
	};

	// get co
	let co_reducer = context.try_co_reducer(&co).await?;
	let co_reducer_state = co_reducer.reducer_state().await;
	let storage = co_reducer.storage();

	// get identity
	let identity_resolver = context.private_identity_resolver().await?;
	let identity = identity_resolver.resolve_private(&identity).await?;

	// get instance
	let instance = {
		let mut instances = epic.instances.lock().unwrap();
		match instances.get(&co) {
			Some(instance) => instance.clone(),
			None => {
				let instance = PushHeads::new(
					context.date().clone(),
					network,
					context.tasks(),
					co.clone(),
					co_reducer.context.has_feature(&CoReducerFeature::Encryption),
				)?;
				instances.insert(co.clone(), instance.clone());
				instance
			},
		}
	};

	// push
	instance.changed(&storage, co_reducer_state, identity).await?;

	Ok(())
}
