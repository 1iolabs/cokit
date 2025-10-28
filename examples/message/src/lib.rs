use cid::Cid;
use co_api::{
	sync_api::{Context, Reducer, StorageExt},
	CoMetadata, Date, Did, Link, Metadata, ReducerAction, Storage,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum MessageVersion {
	#[default]
	V1 = 1,
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
				let role = storage.get_value(link).unwrap_or(Role::None);
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

	fn reduce(self, action: &ReducerAction<Self::Action>, context: &mut dyn Context) -> Self {
		match &action.payload {
			MessageAction::SetName(name) => {
				if Permission::Name.has(context.storage(), &self, &action.from) {
					MessageState { name: name.clone(), ..self }
				} else {
					self
				}
			},
			MessageAction::Message => {
				let participants = match self.participants.get(&action.from) {
					Some(_) => self.participants,
					None => {
						let mut new = self.participants.clone();
						new.insert(action.from.clone(), context.storage_mut().set_value(&Role::Participant));
						new
					},
				};
				MessageState { participants, message_count: self.message_count + 1, ..self }
			},
			MessageAction::Pin(id) => {
				let mut pinned = self.pinned.clone();
				pinned.push(*id);
				MessageState { pinned, ..self }
			},
			MessageAction::SetRole(did, role_link) => {
				let from_role_link_option = self.participants.get(&action.from);
				match from_role_link_option {
					Some(from_role_link) => {
						let role = context.storage().get_value(from_role_link).expect("valid role");
						if role == Role::Admin {
							let mut participants = self.participants.clone();
							participants.insert(did.clone(), *role_link);
							MessageState { participants, ..self }
						} else {
							self
						}
					},
					None => self,
				}
			},
		}
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
		if !Permission::Name.has(context.storage.as_ref(), self, &context.from) {
			return Err(CallError::Permission);
		}
		self.name = name;
		Ok(())
	}
}

// impl MessageState {
// 	// #[api]
// 	pub fn set_name(name: String) -> SetNameEvent {}
// }

#[cfg(all(feature = "core", target_arch = "wasm32", target_os = "unknown"))]
#[no_mangle]
pub extern "C" fn state() {
	co_api::sync_api::reduce::<MessageState>()
}
