// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

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
