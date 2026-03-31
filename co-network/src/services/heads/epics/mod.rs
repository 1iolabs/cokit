// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::services::heads::{
	actor::{HeadsContext, HeadsState},
	HeadsAction,
};
use co_actor::{Epic, EpicExt, TracingEpic};
use co_primitives::Tags;

mod listen;
mod publish;
mod subscribe;
mod unsubscribe;

pub fn epic(tags: Tags) -> impl Epic<HeadsAction, HeadsState, HeadsContext> {
	subscribe::subscribe
		.join(unsubscribe::unsubscribe)
		.join(publish::publish)
		.join(listen::listen)
		.join(TracingEpic::new(tags))
}
