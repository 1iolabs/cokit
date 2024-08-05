use crate::{library::create_reducer_action::create_reducer_action, ReducerChangeContext};
use co_identity::Message;
use co_primitives::{CoId, Did, Link, OptionLink, ReducerAction};
use co_storage::{BlockStorage, BlockStorageExt, StorageError};
use futures::Stream;
use libipld::{Cid, Ipld};
use libp2p::PeerId;
use serde::Serialize;
use std::{collections::BTreeSet, ops::Deref, sync::Arc};

#[derive(Debug, Clone)]
pub enum Action {
	/// Push core action.
	CoreActionPush { co: CoId, action: ReducerAction<Ipld> },

	/// Core action has been succesfully processed.
	CoreAction {
		co: CoId,
		context: ReducerChangeContext,
		action: ReducerAction<Ipld>,
		cid: OptionLink<ReducerAction<Ipld>>,
	},

	/// Core action has been failed.
	CoreActionFailure { co: CoId, context: ReducerChangeContext, action: ReducerAction<Ipld>, err: ActionError },

	/// Generic Error.
	Error { err: ActionError },

	/// Send invite request.
	Invite { co: CoId, from: Did, to: Did },

	/// Invite request has been sent to a peer.
	InviteSent { co: CoId, participant: Did, peer: PeerId },

	/// Join request has been sent to a peer.
	JoinSent { co: CoId, heads: BTreeSet<Cid>, participant: Did, peer: PeerId },

	/// Join completed.
	Joined { co: CoId, participant: Did, success: bool },

	/// Send Key Request to co (participants) or specified peer.
	// KeyRequest { co: CoId, key: Option<String>, peer: Option<PeerId> },

	/// Network has been started.
	NetworkStarted,

	/// Received a DIDComm message.
	DidCommReceive { peer: PeerId, message: Message },
}
impl Action {
	pub async fn core_action<S>(
		storage: &S,
		co: CoId,
		context: ReducerChangeContext,
		cid: Link<ReducerAction<Ipld>>,
	) -> Result<Self, StorageError>
	where
		S: BlockStorage + Send + Sync + 'static,
	{
		Ok(Self::CoreAction { co, context, action: storage.get_value(&cid).await?, cid: cid.into() })
	}

	/// Map result to action.
	pub fn map_error<E>(item: Result<Action, E>) -> Self
	where
		E: Into<anyhow::Error>,
	{
		match item {
			Ok(item) => item,
			Err(err) => err.into().into(),
		}
	}

	/// Map result of vec of actions into action stream.
	pub fn map_error_stream<E>(item: Result<impl IntoIterator<Item = Action>, E>) -> impl Stream<Item = Action>
	where
		E: Into<anyhow::Error>,
	{
		async_stream::stream! {
			match item {
				Ok(items) => {
					for item in items {
						yield item;
					}
				},
				Err(err) => yield err.into().into(),
			}
		}
	}

	/// Utilit to create [`Action::CoreActionPush`] actions.
	pub fn push(co: impl Into<CoId>, from: impl Into<Did>, core: impl Into<String>, payload: impl Serialize) -> Action {
		let reducer_action = match create_reducer_action(from, core.into(), payload) {
			Ok(a) => a,
			Err(err) => {
				return Action::Error { err: err.into() };
			},
		};
		Action::CoreActionPush { co: co.into(), action: reducer_action }
	}
}
impl From<anyhow::Error> for Action {
	fn from(value: anyhow::Error) -> Self {
		Action::Error { err: value.into() }
	}
}

#[derive(Debug, Clone)]
pub enum ActionError {
	Serialized { message: String },
	Native { err: Arc<anyhow::Error> },
}
impl From<anyhow::Error> for ActionError {
	fn from(value: anyhow::Error) -> Self {
		Self::Native { err: Arc::new(value) }
	}
}
impl std::error::Error for ActionError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match &self {
			ActionError::Serialized { message: _ } => None,
			ActionError::Native { err } => Some(err.deref().deref()),
		}
	}
}
impl std::fmt::Display for ActionError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ActionError::Serialized { message } => write!(f, "{}", message),
			ActionError::Native { err } => write!(f, "{}", err),
		}
	}
}
