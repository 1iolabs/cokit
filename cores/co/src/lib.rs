use co_primitives::{Did, ReducerAction, Tags};
use co_wasm_api::{reduce, Context, Reducer};
use libipld::Cid;
use libp2p_core::PeerId;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::{BTreeMap, BTreeSet};

// #[co_wasm_api::State]
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Co {
	/// CO UUID.
	pub id: Vec<u8>,

	/// CO Tags.
	pub tags: Tags,

	/// CO Name.
	pub name: String,

	/// CO Current heads.
	pub heads: BTreeSet<Cid>,

	/// CO Participants.
	pub participants: BTreeMap<Did, Participant>,

	/// CO Streams with the associated state reference.
	///
	/// Key: Core Instance
	pub cores: BTreeMap<String, Core>,

	/// CO Encryption Keys.
	/// The first (index: 0) key is the active key.
	/// Keys are normally stored in the Local CO.
	pub keys: Option<Vec<Key>>,

	/// CO known peers.
	// #[co_wasm_api::Dag]
	pub peers: BTreeSet<PeerId>,
	//
	// /// CO settings.
	// /// Normally settings are formatted with an dot notation.
	// /// Formats:
	// /// - `<core_name>.<setting_name>`
	// /// - `<core_name>.<setting_namespaces>.<setting_name>`
	// ///
	// /// Examples:
	// /// - `co.`
	// ///
	// /// Todo: Use Tags (Vec<(String, Ipld)>)?
	// pub settings: BTreeMap<String, Ipld>,
}

// #[co_wasm_api::Data]
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Core {
	/// The CID of the core binary.
	pub binary: Cid,

	/// COre Tags.
	pub tags: Tags,

	// /// The CID of the permission binaries.
	// /// If multiple binaries are specified they will be executed in parallel.
	// /// If not specified the permissions are managed by the reducer.
	// pub permission_binaries: Vec<Cid>,
	/// The latest stream state.
	pub state: Cid,
}

// #[co_wasm_api::Data]
#[derive(Debug, Clone, Serialize_repr, Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum Architecture {
	Wasm = 0,
}

// #[co_wasm_api::Data]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Participant {
	/// The participant DID.
	pub did: Did,

	/// Participant state.
	pub state: ParticipantState,

	/// Participant tags.
	pub tags: Tags,
	// /// Participant permissions.
	// /// Only used when the default permission check is used.
	// pub permissions: BTreeSet<Permission>,
	//
	// /// Additional participant settings and metadata.
	// pub settings: BTreeMap<String, Ipld>,
}

// // #[co_wasm_api::Data]
// #[derive(Debug, Clone, Serialize_repr, Deserialize_repr, PartialEq, Eq, PartialOrd, Ord)]
// #[non_exhaustive]
// #[repr(u8)]
// pub enum Permission {
// 	ParticipantInvite = 0,
// 	/// Change participants permission and settings.
// 	ParticipantChange = 1,
// 	SettingChange = 2,
// 	CoreCreate = 3,
// 	CoreRemove = 4,
// 	KeyRotate = 5,
// 	NameChange = 6,
// }
// impl Permission {
// 	pub fn defaults() -> BTreeSet<Permission> {
// 		let mut result = BTreeSet::new();
// 		result.insert(Permission::CoreCreate);
// 		result
// 	}
// }

#[derive(Debug, Clone, Serialize_repr, Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum ParticipantState {
	/// Active participant.
	Active = 0,

	/// Invited participant.
	Invite = 1,

	/// Inactive (Removed, Resigned, Banned, ...) participant.
	Inactive = 2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Key {
	pub id: String,
	pub state: KeyState,
}

#[derive(Debug, Clone, Serialize_repr, Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum KeyState {
	Inactive = 0,
	Active = 1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoAction {
	// #[serde(rename = "i")]
	Invite { did: Did, tags: Tags },
	Join { did: Did },
	Heads { heads: BTreeSet<Cid> },
}

// impl Co {
// 	pub fn set_name(mut self, event: &ReducerAction<CoAction>, name: String) -> Result<Self, anyhow::Error> {
// 		let permissions = self.participants.get(&event.from).map(|p| p.permissions).unwrap_or_default();
// 		if !permissions.contains(&Permission::NameChange) {
// 			return Err(anyhow!("no permission"));
// 		}
// 		self.name = name;
// 		Ok(self)
// 	}
// }

impl Reducer for Co {
	type Action = CoAction;

	fn reduce(self, event: &ReducerAction<Self::Action>, _: &mut dyn Context) -> Self {
		let mut result = self;
		match &event.payload {
			CoAction::Invite { did, tags } =>
				if !result.participants.contains_key(did) {
					result.participants.insert(
						did.clone(),
						Participant { did: did.clone(), state: ParticipantState::Invite, tags: tags.clone() },
					);
				},
			CoAction::Join { did } =>
				if let Some(participant) = result.participants.get_mut(did) {
					participant.state = ParticipantState::Active;
				},
			CoAction::Heads { heads } => {
				result.heads = heads.clone();
			},
		}
		result
	}
}

// pub extern "C" fn permission() -> bool {}

#[no_mangle]
pub extern "C" fn state() {
	reduce::<Co>()
}
