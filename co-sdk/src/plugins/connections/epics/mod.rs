use super::{ConnectionAction, ConnectionState};
use crate::{
	actor::{Epic, EpicExt, TracingEpic},
	CoContext,
};
use co_primitives::Tags;

mod connect;
mod network_resolve;

pub fn epic(tags: Tags) -> impl Epic<ConnectionAction, ConnectionState, CoContext> {
	connect::ConnectEpic()
		.join(network_resolve::NetworkResolveEpic())
		.join(TracingEpic(tags))
}
