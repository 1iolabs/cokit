use crate::{
	library::create_reducer_action::new_reducer_action, services::reducer::FlushInfo,
	types::message::heads::HeadsMessage, CoDate, CoStorage, ReducerChangeContext,
};
use co_identity::Message;
use co_network::didcomm::EncodedMessage;
use co_primitives::{CoId, Did, Link, OptionLink, ReducerAction};
use co_storage::{BlockStorage, BlockStorageExt, StorageError};
use futures::{stream::once, Stream, StreamExt};
use ipld_core::ipld::Ipld;
use libp2p::PeerId;
use serde::Serialize;
use std::{
	future::{ready, Future},
	ops::Deref,
	sync::Arc,
};

#[derive(Debug, Clone)]
pub enum Action {
	/// Push core action.
	CoreActionPush { co: CoId, action: ReducerAction<Ipld> },

	/// Core action has been succesfully processed (and flushed).
	CoreAction {
		co: CoId,
		storage: CoStorage,
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
	JoinSent { co: CoId, encrypted: bool, participant: Did, peer: PeerId },

	/// Join completed.
	Joined { co: CoId, participant: Did, success: bool, peer: Option<PeerId> },

	/// Send Key Request to co (participants) or specified peer.
	// KeyRequest { co: CoId, key: Option<String>, peer: Option<PeerId> },

	/// Start network.
	NetworkStart { force_new_peer_id: bool },

	/// Network has been started.
	NetworkStarted,

	/// Send a DIDComm message.
	DidCommSend {
		/// The message id for reference.
		message_id: String,
		/// Peer to send the message to.
		peer: PeerId,
		/// The message.
		message: EncodedMessage,
	},

	/// Sent result of the DIDComm message.
	DidCommSent {
		/// The message id for reference.
		message_id: String,
		/// Peer to send the message to.
		peer: PeerId,
		/// The send result.
		result: Result<(), ActionError>,
	},

	/// Received a DIDComm message.
	DidCommReceive { peer: PeerId, message: Message },

	/// Received a HeadsMessage.
	HeadsMessageReceived { from: Option<Did>, peer: PeerId, message_id: String, message: HeadsMessage },

	/// Send a DIDComm message to all connectable co peers.
	CoDidCommSend {
		/// The Co to send the message to.
		co: CoId,

		/// The message id for reference.
		message_id: String,

		/// The message.
		message: EncodedMessage,
	},

	/// Staged changes to a CO has been flushed.
	CoFlush {
		/// The flushed CO.
		co: CoId,

		/// Flush details.
		info: FlushInfo,
	},

	/// Stage a action and dispatch after flush.
	CoStaged { co: CoId, action: Box<Action> },

	/// Co has been opened.
	CoOpen {
		/// The opened CO.
		co: CoId,

		/// Whether the co has a network feature.
		network: bool,
	},

	/// Co has been closed.
	CoClose {
		/// The opened CO.
		co: CoId,
	},
}
impl Action {
	pub async fn core_action<S>(
		storage: &S,
		co: CoId,
		context: ReducerChangeContext,
		cid: Link<ReducerAction<Ipld>>,
	) -> Result<Self, StorageError>
	where
		S: BlockStorage + Into<CoStorage> + Clone + Send + Sync + 'static,
	{
		Ok(Self::CoreAction {
			co,
			context,
			storage: storage.clone().into(),
			action: storage.get_value(&cid).await?,
			cid: cid.into(),
		})
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

	/// Map error to action ignoning the result value.
	pub fn to_error<E>(item: Result<(), E>) -> Option<Self>
	where
		E: Into<anyhow::Error>,
	{
		match item {
			Ok(_) => None,
			Err(err) => Some(err.into().into()),
		}
	}

	/// Map error to action ignoning the result value.
	pub async fn filter_map_error(item: Result<(), anyhow::Error>) -> Option<Self> {
		match item {
			Ok(_) => None,
			Err(err) => Some(err.into()),
		}
	}

	/// Map error to action ignoning the result value.
	pub fn ignore_elements<T>(
		stream: impl Stream<Item = Result<T, anyhow::Error>> + 'static,
	) -> impl Stream<Item = Result<Action, anyhow::Error>> + 'static {
		stream.filter_map(|item| {
			ready(match item {
				Ok(_) => None,
				Err(err) => Some(Result::<Action, anyhow::Error>::Err(err)),
			})
		})
	}

	/// Map error to action ignoning the result value.
	pub fn future_ignore_elements<T>(
		fut: impl Future<Output = Result<T, anyhow::Error>> + 'static,
	) -> impl Stream<Item = Result<Action, anyhow::Error>> + 'static {
		Self::ignore_elements(once(fut))
	}

	/// Utility to create [`Action::CoreActionPush`] actions.
	pub fn push(
		co: impl Into<CoId>,
		from: impl Into<Did>,
		core: impl Into<String>,
		payload: impl Serialize,
		date: &impl CoDate,
	) -> Action {
		let reducer_action = match new_reducer_action(from, core.into(), payload, date) {
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
