// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

#[cfg(feature = "network")]
use crate::library::network_queue::TaskState;
use crate::{
	library::create_reducer_action::new_reducer_action, services::reducer::FlushInfo, CoStorage, ReducerChangeContext,
};
use cid::Cid;
use co_identity::PrivateIdentityBox;
#[cfg(feature = "network")]
use co_identity::{DidCommHeader, Message};
#[cfg(feature = "network")]
use co_network::{EncodedMessage, HeadsMessage, NetworkSettings, PeerId};
use co_primitives::{Block, BlockSerializer, CoDate, CoId, Did, Link, Network, ReducerAction, Tags};
use co_storage::{BlockStorage, BlockStorageExt, StorageError};
use futures::{stream::once, Stream, StreamExt};
use ipld_core::ipld::Ipld;
use serde::{Deserialize, Serialize};
use std::{
	collections::{BTreeMap, BTreeSet},
	future::{ready, Future},
	ops::Deref,
	sync::Arc,
};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Action {
	/// Push core action.
	CoreActionPush { co: CoId, action: ReducerAction<Ipld> },

	/// Core action has been successfully processed (and flushed).
	CoreAction {
		co: CoId,
		storage: CoStorage,
		context: ReducerChangeContext,
		action: ReducerAction<Ipld>,
		cid: Link<ReducerAction<Ipld>>,
		head: Cid,
	},

	/// Core action has been failed.
	CoreActionFailure { co: CoId, context: ReducerChangeContext, action: ReducerAction<Ipld>, err: ActionError },

	/// Generic Error.
	Error { err: ActionError },

	/// Send invite request.
	Invite { co: CoId, from: Did, to: Did },

	/// Invite request has been sent to a peer.
	#[cfg(feature = "network")]
	InviteSent { co: CoId, to: Did, peer: PeerId },

	/// Join completed.
	#[cfg(feature = "network")]
	Joined { co: CoId, participant: Did, success: bool, peer: Option<PeerId> },

	/// Send a Key Request to a co or specified network.
	#[cfg(feature = "network")]
	KeyRequest(KeyRequestAction),

	/// Key Request has completed.
	#[cfg(feature = "network")]
	KeyRequestComplete(KeyRequestAction, Result<String, ActionError>),

	/// Start network.
	#[cfg(feature = "network")]
	NetworkStart(NetworkSettings),

	/// Network has been started.
	#[cfg(feature = "network")]
	NetworkStartComplete(Result<(), ActionError>),

	/// Send a contact request.
	Contact(ContactAction),

	/// Contact request send result.
	ContactSent(ContactAction, Result<(), ActionError>),

	/// Send a DIDComm message.
	#[cfg(feature = "network")]
	DidCommSend {
		/// The message header for reference.
		message_header: DidCommHeader,
		/// Peer to send the message to.
		peer: PeerId,
		/// The message.
		message: EncodedMessage,
	},

	/// Sent result of the DIDComm message.
	#[cfg(feature = "network")]
	DidCommSent {
		/// The message header for reference.
		message_header: DidCommHeader,
		/// Peer to send the message to.
		peer: PeerId,
		/// The send result.
		result: Result<(), ActionError>,
	},

	/// Received a DIDComm message.
	///
	/// # Security
	/// It is not proofed that the sender (peer) is the producer of the message.
	/// If such a proof is needed it must be included in a signed message.
	#[cfg(feature = "network")]
	DidCommReceive { peer: PeerId, message: Message },

	/// Received a HeadsMessage.
	#[cfg(feature = "network")]
	HeadsMessageReceived(HeadsMessageReceivedAction),

	/// HeadsMessage has been processed.
	#[cfg(feature = "network")]
	HeadsMessageComplete(HeadsMessageReceivedAction, Result<(), HeadsError>),

	/// Connect to Co and send message (DidCommSent) to the first peer connectable.
	#[cfg(feature = "network")]
	CoDidCommSend(CoDidCommSendAction),

	/// DidComm message send result
	/// Emitted once per [`Action::CoDidCommSend`].
	#[cfg(feature = "network")]
	CoDidCommSent {
		// The message.
		message: CoDidCommSendAction,
		/// Peers the message has sent to or error.
		/// If the peers list is empty no peer could be connected.
		result: Result<BTreeSet<PeerId>, ActionError>,
	},

	/// Staged changes to a CO has been flushed.
	CoFlush {
		/// The flushed CO.
		co: CoId,

		/// Flush details.
		info: FlushInfo,
	},

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

	/// Add task to network queue.
	NetworkTaskQueue { co: CoId, task_id: String, task_type: String, task_name: String, task: Block },

	/// Execute network queue task.
	#[cfg(feature = "network")]
	NetworkTaskExecute { co: CoId, task_id: String, task_type: String, task: Block },

	/// Execute network queue task has been completed.
	#[cfg(feature = "network")]
	NetworkTaskExecuteComplete { co: CoId, task_id: String, task_state: TaskState },

	/// Network Queue Process
	NetworkQueueProcess {
		/// Only process given co.
		co: Option<CoId>,

		/// Retry count.
		retry: u32,
	},

	/// Network Queue Process Complte
	NetworkQueueProcessComplete {
		/// Only process given co.
		co: Option<CoId>,

		/// Whether the queue is now empty (if specified for the given `co`).
		is_empty: bool,

		/// Retry count.
		retry: u32,
	},

	/// Request a block from network.
	NetworkBlockGet(NetworkBlockGetAction),

	/// Request a block from network complete.
	NetworkBlockGetComplete(NetworkBlockGetAction, Result<(), StorageError>),

	/// Resolve a private identity.
	ResolvePrivateIdentity(ResolvePrivateIdentityAction),

	/// Resolve a private identity complete.
	ResolvePrivateIdentityComplete(ResolvePrivateIdentityAction, Result<PrivateIdentityBox, ActionError>),

	/// Notification.
	Notify(NotifyAction),
}
impl Action {
	pub async fn core_action<S>(
		storage: &S,
		co: CoId,
		context: ReducerChangeContext,
		cid: Link<ReducerAction<Ipld>>,
		head: Cid,
	) -> Result<Self, StorageError>
	where
		S: BlockStorage + Into<CoStorage> + Clone + Send + Sync + 'static,
	{
		Ok(Self::CoreAction {
			co,
			context,
			storage: storage.clone().into(),
			action: storage.get_value(&cid).await?,
			cid,
			head,
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

	pub fn network_task_queue(
		co: CoId,
		task_id: impl Into<String>,
		task_type: impl Into<String>,
		task_name: impl Into<String>,
		task: &impl Serialize,
	) -> Result<Action, anyhow::Error> {
		Ok(Action::NetworkTaskQueue {
			co,
			task_id: task_id.into(),
			task_type: task_type.into(),
			task_name: task_name.into(),
			task: BlockSerializer::default().serialize(task)?,
		})
	}
}
impl From<anyhow::Error> for Action {
	fn from(value: anyhow::Error) -> Self {
		Action::Error { err: value.into() }
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(into = "String", from = "String")]
pub enum ActionError {
	Serialized { message: String },
	Native { err: Arc<anyhow::Error> },
}
impl From<anyhow::Error> for ActionError {
	fn from(value: anyhow::Error) -> Self {
		Self::Native { err: Arc::new(value) }
	}
}
impl From<String> for ActionError {
	fn from(value: String) -> Self {
		Self::Serialized { message: value }
	}
}
impl From<ActionError> for String {
	fn from(value: ActionError) -> Self {
		match value {
			ActionError::Serialized { message } => message,
			ActionError::Native { err } => err.to_string(),
		}
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

/// Contact request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContactAction {
	/// Sender of the contact request.
	pub from: Did,

	/// Receiver of the contact request.
	pub to: Did,

	/// The subject of the contact request.
	/// Usually the invite link or token.
	pub sub: Option<String>,

	/// Explicit networks to use. If empty, resolved from the recipient's DID.
	#[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
	pub networks: BTreeSet<Network>,

	/// Additional fields.
	pub fields: BTreeMap<String, String>,
}

#[cfg(feature = "network")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyRequestAction {
	/// The CO.
	pub co: CoId,

	/// The parent co of the `co`.
	pub parent_co: CoId,

	/// The Key URI. If not specified the latest key is retrived.
	pub key: Option<String>,

	/// The DID to use for the request. If not specified the network identity is used.
	pub from: Option<Did>,

	/// Specific networks to use. If not specified the Co network settings are used.
	pub network: Option<BTreeSet<Network>>,
}

/// Send a DIDComm message to all connectable co peers.
#[cfg(feature = "network")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoDidCommSendAction {
	/// The Co to send the message to.
	pub co: CoId,

	/// Networks to use.
	/// If no networks are specified they are resolved from the Co.
	pub networks: BTreeSet<Network>,

	/// Notification when sent has been successfully done.
	pub notification: Option<NotifyAction>,

	/// Message tags. Used for internal tracking.
	pub tags: Tags,

	/// The message sender for reference.
	pub message_from: Did,

	/// The message header for reference.
	pub message_header: DidCommHeader,

	/// The message.
	pub message: EncodedMessage,
}

/// Notification. This indicates state updates to previous actions.
/// Serializable to allow to delay them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum NotifyAction {
	/// A join message has been sent.
	JoinSent {
		/// The joined participant (us/from).
		participant: Did,
		/// If the joined Co is encrypted.
		encrypted: bool,
	},

	/// A invite message has been sent.
	InviteSent {
		/// The invited participant.
		to: Did,
	},
}

/// Received a HeadsMessage.
#[cfg(feature = "network")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HeadsMessageReceivedAction {
	/// The Co to send the message to.
	pub co: CoId,

	/// The DID of the sender. If set is must be validated.
	pub from: Option<Did>,

	/// The trusted PeerId of the sender.
	pub from_peer: Option<PeerId>,

	/// The PeerId of the sender from which we received the message.
	pub peer: PeerId,

	/// The message id.
	pub message_id: String,

	/// The message payload.
	pub message: HeadsMessage,

	/// Message tags. Used for internal tracking.
	pub tags: Tags,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum HeadsError {
	/// Transient/Retryable error.
	#[error("Transient Heads Error")]
	Transient(#[from] ActionError),

	/// Permanent error.
	#[error("Permanent Heads Error")]
	Permanent(#[source] ActionError),
}
impl From<anyhow::Error> for HeadsError {
	fn from(value: anyhow::Error) -> Self {
		HeadsError::Transient(value.into())
	}
}

/// Request a block from network .
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct NetworkBlockGetAction {
	/// The Co to send the message to.
	pub co: CoId,

	/// The parent co of the `co`.
	pub parent_co: CoId,

	/// The Cid of the block to get.
	pub cid: Cid,
}

/// Request a private identity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum ResolvePrivateIdentityAction {
	Identity { identity: Did },
	NetworkIdentity { parent_co: CoId, co: CoId },
}
