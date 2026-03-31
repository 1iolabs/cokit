// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::{action::DiscoveryAction, actor::DiscoveryContext, state::DiscoveryState};
use co_actor::{Epic, MergeEpic, TracingEpic};
use co_primitives::Tags;

mod connect;
mod did_listen;
mod did_publish;
mod did_subscribe;
mod did_unsubscribe;
mod mesh_peers;
mod timeout;

pub fn epic(tags: Tags) -> impl Epic<DiscoveryAction, DiscoveryState, DiscoveryContext> {
	MergeEpic::new()
		.join(connect::dial_epic)
		.join(connect::send_resolve_epic)
		.join(did_subscribe::did_subscribe_epic)
		.join(did_unsubscribe::did_unsubscribe_epic)
		.join(did_publish::did_publish_epic)
		.join(did_listen::DidListenEpic::new())
		.join(mesh_peers::mesh_peers_epic)
		.join(timeout::TimeoutEpic::new())
		.join(TracingEpic::new(tags))
}
