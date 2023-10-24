use crate::types::{
	cid::Link,
	reducer::{Context, Reducer, ReducerAction},
	storage::Storage,
	Date,
};
use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap as Map;

type Did = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageState {
	#[serde(rename = "v")]
	version: MessageVersion,

	#[serde(rename = "n")]
	name: String,

	#[serde(rename = "m")]
	message_count: u64,

	#[serde(rename = "p", skip_serializing_if = "Vec::is_empty")]
	pinned: Vec<Cid>,

	#[serde(rename = "r", skip_serializing_if = "Map::is_empty")]
	participants: Map<Did, Link<Role>>,
}

impl Default for MessageState {
	fn default() -> Self {
		Self {
			version: Default::default(),
			name: Default::default(),
			message_count: Default::default(),
			pinned: Default::default(),
			participants: Default::default(),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageVersion {
	V1 = 1,
}
impl Default for MessageVersion {
	fn default() -> Self {
		MessageVersion::V1
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
	pub fn has(&self, storage: &dyn Storage, state: &MessageState, participant: &Did) -> bool {
		match state.participants.get(participant) {
			Some(link) => {
				let role = link.resolve(storage).unwrap_or(Role::None);
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
	None,
	Custom { name: String, permissions: Vec<Permission> },
	Participant,
	Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageAction {
	SetName(String),
	Message,
	Pin(Cid),
	SetRole(Did, Link<Role>),
}

impl Reducer for MessageState {
	type Action = MessageAction;

	fn reduce(self, action: &ReducerAction<Self::Action>, context: &Context) -> Self {
		let mut state = self;
		match &action.payload {
			MessageAction::SetName(name) =>
				if Permission::Name.has(context.storage.as_ref(), &state, &action.from) {
					state = MessageState { name: name.clone(), ..state };
				},
			MessageAction::Message => todo!(),
			MessageAction::Pin(_) => todo!(),
			MessageAction::SetRole(_, _) => todo!(),
		}
		state
	}
}

pub struct CallContext {
	pub storage: Box<dyn Storage>,
	pub from: Did,
	pub time: Date,
}

pub enum CallError {
	Permission,
}

impl MessageState {
	pub fn set_name(&mut self, context: &CallContext, name: String) -> Result<(), CallError> {
		if !Permission::Name.has(context.storage.as_ref(), &self, &context.from) {
			return Err(CallError::Permission)
		}
		self.name = name;
		Ok(())
	}
}

// impl MessageState {
// 	// #[api]
// 	pub fn set_name(name: String) -> SetNameEvent {}
// }
