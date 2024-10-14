use super::{ConnectionAction, ConnectionState};
use crate::{
	actor::{Epic, EpicExt},
	CoContext,
};

mod connect;
mod network_resolve;

pub fn epic() -> impl Epic<ConnectionAction, ConnectionState, CoContext> {
	connect::ConnectEpic().join(network_resolve::NetworkResolveEpic())
}
