use co_api::{reduce, Context, Did, Reducer, ReducerAction, Tags};
use libipld::Cid;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::{BTreeMap, BTreeSet};

// #[co_api::State]
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

	/// CO known peers
	/// See: libp2p::PeerId
	// #[co_api::Dag]
	pub peers: BTreeSet<Vec<u8>>,
}

// #[co_api::Data]
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Core {
	/// The CID of the core binary.
	pub binary: Cid,

	/// COre Tags.
	pub tags: Tags,

	/// The latest stream state.
	pub state: Option<Cid>,
}

// #[co_api::Data]
#[derive(Debug, Clone, Serialize_repr, Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum Architecture {
	Wasm = 0,
}

// #[co_api::Data]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Participant {
	/// The participant DID.
	pub did: Did,

	/// Participant state.
	pub state: ParticipantState,

	/// Participant tags.
	pub tags: Tags,
}

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
	Create { id: Vec<u8>, name: String, cores: BTreeMap<String, Core>, participants: BTreeMap<Did, Participant> },
	Heads { heads: BTreeSet<Cid> },
	TagsInsert { tags: Tags },
	TagsRemove { tags: Tags },
	ParticipantInvite { participant: Did, tags: Tags },
	ParticipantJoin { participant: Did },
	ParticipantTagsInsert { participant: Did, tags: Tags },
	ParticipantTagsRemove { participant: Did, tags: Tags },
	CoreCreate { core: String, binary: Cid, tags: Tags },
	CoreRemove { core: String },
	CoreChange { core: String, state: Option<Cid> },
	CoreTagsInsert { core: String, tags: Tags },
	CoreTagsRemove { core: String, tags: Tags },
}

impl Reducer for Co {
	type Action = CoAction;

	fn reduce(self, event: &ReducerAction<Self::Action>, _: &mut dyn Context) -> Self {
		let mut result = self;
		match &event.payload {
			CoAction::Create { id, name, cores, participants } => {
				// only allowed for empty COs
				// id can not be changed afterwards
				if result.id.is_empty() {
					result.id = id.to_owned();
					result.name = name.to_owned();
					result.cores = cores.to_owned();
					result.participants = participants.to_owned();
				}
			},
			CoAction::ParticipantInvite { participant, tags } =>
				if !result.participants.contains_key(participant) {
					result.participants.insert(
						participant.clone(),
						Participant { did: participant.clone(), state: ParticipantState::Invite, tags: tags.clone() },
					);
				},
			CoAction::ParticipantJoin { participant } =>
				if let Some(participant) = result.participants.get_mut(participant) {
					participant.state = ParticipantState::Active;
				},
			CoAction::Heads { heads } => {
				result.heads = heads.clone();
			},
			CoAction::CoreCreate { core, binary, tags } =>
				if !result.cores.contains_key(core) {
					result
						.cores
						.insert(core.clone(), Core { binary: binary.clone(), tags: tags.clone(), state: None });
				},
			CoAction::CoreRemove { core } => {
				result.cores.remove(core);
			},
			CoAction::ParticipantTagsInsert { participant, tags } => {
				if let Some(participant) = result.participants.get_mut(participant) {
					participant.tags.append(&mut tags.clone());
				}
			},
			CoAction::ParticipantTagsRemove { participant, tags } => {
				if let Some(participant) = result.participants.get_mut(participant) {
					participant.tags.clear(Some(tags));
				}
			},
			CoAction::CoreChange { core, state } =>
				if let Some(core) = result.cores.get_mut(core) {
					core.state = state.clone();
				},
			CoAction::CoreTagsInsert { core, tags } =>
				if let Some(core) = result.cores.get_mut(core) {
					core.tags.append(&mut tags.clone());
				},
			CoAction::CoreTagsRemove { core, tags } =>
				if let Some(core) = result.cores.get_mut(core) {
					core.tags.clear(Some(tags));
				},
			CoAction::TagsInsert { tags } => {
				result.tags.append(&mut tags.clone());
			},
			CoAction::TagsRemove { tags } => {
				result.tags.clear(Some(tags));
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
