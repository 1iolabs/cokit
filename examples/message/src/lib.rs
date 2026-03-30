// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_api::{
	co, BlockStorageExt, CoMetadata, CoreBlockStorage, Date, Did, Link, Metadata, OptionLink, Reducer, ReducerAction,
};
use std::collections::BTreeMap;

#[co(state)]
pub struct MessageState {
	#[serde(rename = "v")]
	pub version: MessageVersion,

	#[serde(rename = "n")]
	pub name: String,

	#[serde(rename = "m")]
	pub message_count: u64,

	#[serde(rename = "p", default, skip_serializing_if = "Vec::is_empty")]
	pub pinned: Vec<Cid>,

	#[serde(rename = "r", default, skip_serializing_if = "BTreeMap::is_empty")]
	pub participants: BTreeMap<Did, Link<Role>>,
}

impl CoMetadata for MessageState {
	fn metadata() -> Vec<co_api::Metadata> {
		vec![Metadata::External(vec!["pinned".to_string()])]
	}
}

#[co]
#[derive(Default)]
pub enum MessageVersion {
	#[default]
	V1 = 1,
}

#[co]
pub enum Permission {
	Send,
	Read,
	Forward,
	Reply,
	Call,
	Download,
	Upload,
	Name,
	Pin,
}
impl Permission {
	pub async fn has(&self, storage: &CoreBlockStorage, state: &MessageState, participant: &Did) -> bool {
		match state.participants.get(participant) {
			Some(link) => {
				let role = storage.get_value(link).await.unwrap_or(Role::None);
				match role {
					Role::None => false,
					Role::Custom { name: _, permissions } => permissions.contains(self),
					Role::Participant => match self {
						Permission::Send => true,
						Permission::Read => true,
						Permission::Forward => true,
						Permission::Reply => true,
						Permission::Call => true,
						Permission::Download => true,
						Permission::Upload => true,
						Permission::Name => false,
						Permission::Pin => false,
					},
					Role::Admin => true,
				}
			},
			None => false,
		}
	}
}

#[co]
pub enum Role {
	None,
	Custom { name: String, permissions: Vec<Permission> },
	Participant,
	Admin,
}

#[co]
pub enum MessageAction {
	SetName(String),
	Message,
	Pin(Cid),
	SetRole(Did, Link<Role>),
}

impl Reducer<MessageAction> for MessageState {
	async fn reduce(
		state: OptionLink<Self>,
		event: Link<ReducerAction<MessageAction>>,
		storage: &CoreBlockStorage,
	) -> Result<Link<Self>, anyhow::Error> {
		let action = storage.get_value(&event).await?;
		let current = storage.get_value_or_default(&state).await?;
		let next = match &action.payload {
			MessageAction::SetName(name) => {
				if Permission::Name.has(storage, &current, &action.from).await {
					MessageState { name: name.clone(), ..current }
				} else {
					current
				}
			},
			MessageAction::Message => {
				let participants = match current.participants.get(&action.from) {
					Some(_) => current.participants,
					None => {
						let mut new = current.participants.clone();
						new.insert(action.from.clone(), storage.set_value(&Role::Participant).await?);
						new
					},
				};
				MessageState { participants, message_count: current.message_count + 1, ..current }
			},
			MessageAction::Pin(id) => {
				let mut pinned = current.pinned.clone();
				pinned.push(*id);
				MessageState { pinned, ..current }
			},
			MessageAction::SetRole(did, role_link) => {
				let from_role_link_option = current.participants.get(&action.from);
				match from_role_link_option {
					Some(from_role_link) => {
						let role: Role = storage.get_value(from_role_link).await?;
						if role == Role::Admin {
							let mut participants = current.participants.clone();
							participants.insert(did.clone(), *role_link);
							MessageState { participants, ..current }
						} else {
							current
						}
					},
					None => current,
				}
			},
		};
		Ok(storage.set_value(&next).await?)
	}
}

pub struct CallContext {
	pub storage: CoreBlockStorage,
	pub from: Did,
	pub time: Date,
}

pub enum CallError {
	Permission,
}

impl MessageState {
	pub async fn set_name(&mut self, context: &CallContext, name: String) -> Result<(), CallError> {
		if !Permission::Name.has(&context.storage, self, &context.from).await {
			return Err(CallError::Permission);
		}
		self.name = name;
		Ok(())
	}
}
