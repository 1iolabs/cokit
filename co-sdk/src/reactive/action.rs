use std::{ops::Deref, sync::Arc};

use co_primitives::{CoId, Did, Link, OptionLink, ReducerAction};
use co_storage::{BlockStorage, BlockStorageExt, StorageError};
use libipld::Ipld;
use libp2p::PeerId;

use crate::ReducerChangeContext;

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

	/// Invite request has been sent to a peer.
	Invited { co: CoId, participant: Did, peer: PeerId },
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

	pub fn map_error<E>(item: Result<Action, E>) -> Self
	where
		E: Into<anyhow::Error>,
	{
		match item {
			Ok(item) => item,
			Err(err) => err.into().into(),
		}
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
