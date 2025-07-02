use cid::Cid;
use co_api::{
	co, BlockStorage, BlockStorageExt, CoId, Context, DagSet, DagSetExt, Did, Network, Reducer, ReducerAction,
	SignedEntry, Tags,
};
use std::collections::{BTreeMap, BTreeSet};

#[co(state_sync, guard, no_default)]
#[non_exhaustive]
pub struct Co {
	/// CO UUID.
	pub id: CoId,

	/// CO Tags.
	pub tags: Tags,

	/// CO Name.
	pub name: String,

	/// CO Core Binary.
	pub binary: Cid,

	/// CO Current heads.
	pub heads: BTreeSet<Cid>,

	/// CO Participants.
	pub participants: BTreeMap<Did, Participant>,

	/// CO Streams with the associated state reference.
	///
	/// Key: Core Instance
	pub cores: BTreeMap<String, Core>,

	/// Co Guards.
	pub guards: BTreeMap<String, Guard>,

	/// CO Encryption Keys.
	/// The first (index: 0) key is the active key.
	/// Keys are normally stored in the Local CO.
	pub keys: Option<Vec<Key>>,

	/// CO network services.
	/// See: [`libp2p::PeerId`]
	// #[co_api::Dag]
	pub network: DagSet<Network>,
}
impl Default for Co {
	fn default() -> Self {
		Self {
			id: "".into(),
			tags: Default::default(),
			name: Default::default(),
			binary: Default::default(),
			heads: Default::default(),
			participants: Default::default(),
			cores: Default::default(),
			keys: Default::default(),
			network: Default::default(),
			guards: Default::default(),
		}
	}
}

#[co]
#[derive(Default)]
pub struct Core {
	/// The CID of the core binary.
	pub binary: Cid,

	/// COre Tags.
	pub tags: Tags,

	/// The latest stream state.
	pub state: Option<Cid>,
}

#[co]
pub struct Guard {
	/// The CID of the guard binary.
	pub binary: Cid,

	/// Guard Tags.
	pub tags: Tags,
}

#[co(repr)]
#[repr(u8)]
pub enum Architecture {
	Wasm = 0,
}

#[co]
pub struct Participant {
	/// The participant DID.
	pub did: Did,

	/// Participant state.
	pub state: ParticipantState,

	/// Participant tags.
	pub tags: Tags,
}

#[co(repr)]
#[repr(u8)]
pub enum ParticipantState {
	/// Active participant.
	Active = 0,

	/// Inactive (Removed, Resigned, Banned, ...) participant.
	Inactive = 1,

	/// Invited participant.
	Invite = 2,

	/// Pending participant.
	///
	/// Usually this is a manual Join request.
	/// Pending participants need to be moved into [`ParticipantState::Invite`] state by a participant.
	Pending = 3,
}
impl ParticipantState {
	pub fn is_active(&self) -> bool {
		match self {
			ParticipantState::Active => true,
			_ => false,
		}
	}

	pub fn has_access(&self) -> bool {
		match self {
			ParticipantState::Active => true,
			ParticipantState::Invite => true,
			_ => false,
		}
	}
}

#[co]
pub struct Key {
	pub id: String,
	pub state: KeyState,
}

#[co(repr)]
#[repr(u8)]
pub enum KeyState {
	Inactive = 0,
	Active = 1,
}

#[co]
#[non_exhaustive]
pub struct CreateAction {
	pub id: CoId,
	pub name: String,
	pub cores: BTreeMap<String, Core>,
	pub guards: BTreeMap<String, Guard>,
	pub participants: BTreeMap<Did, Participant>,
	pub key: Option<String>,
	pub binary: Cid,
}
impl CreateAction {
	pub fn new(id: CoId, name: String, binary: Cid) -> Self {
		Self { id, name, binary, ..Default::default() }
	}

	pub fn with_core(mut self, core_name: String, core: Core) -> Self {
		self.cores.insert(core_name, core);
		self
	}

	pub fn with_participant(mut self, participant: Did, tags: Tags) -> Self {
		self.participants.insert(
			participant.clone(),
			Participant { did: participant.clone(), state: ParticipantState::Active, tags },
		);
		self
	}

	pub fn with_key(mut self, key: Option<String>) -> Self {
		self.key = key;
		self
	}
}
impl Default for CreateAction {
	fn default() -> Self {
		Self {
			id: "".into(),
			name: Default::default(),
			cores: Default::default(),
			guards: Default::default(),
			participants: Default::default(),
			key: Default::default(),
			binary: Default::default(),
		}
	}
}

#[co]
#[non_exhaustive]
pub enum CoAction {
	Create(CreateAction),
	Upgrade {
		binary: Cid,
		migrate: Option<Cid>,
	},
	Heads {
		heads: BTreeSet<Cid>,
	},
	TagsInsert {
		tags: Tags,
	},
	TagsRemove {
		tags: Tags,
	},
	ParticipantInvite {
		participant: Did,
		tags: Tags,
	},
	ParticipantJoin {
		participant: Did,
		tags: Tags,
	},
	ParticipantPending {
		participant: Did,
		tags: Tags,
	},
	ParticipantRemove {
		participant: Did,
		tags: Tags,
	},
	ParticipantTagsInsert {
		participant: Did,
		tags: Tags,
	},
	ParticipantTagsRemove {
		participant: Did,
		tags: Tags,
	},
	NetworkInsert {
		network: Network,
	},
	NetworkRemove {
		network: Network,
	},
	CoreCreate {
		core: String,
		binary: Cid,
		tags: Tags,
	},
	CoreRemove {
		core: String,
	},
	CoreChange {
		core: String,
		state: Option<Cid>,
	},
	CoreUpgrade {
		core: String,

		/// The new binary.
		binary: Cid,

		/// Migrate action.
		/// Must deserialize to a action using the new `binary`.
		migrate: Option<Cid>,
	},
	CoreTagsInsert {
		core: String,
		tags: Tags,
	},
	CoreTagsRemove {
		core: String,
		tags: Tags,
	},
	GuardCreate {
		guard: String,
		binary: Cid,
		tags: Tags,
	},
	GuardRemove {
		guard: String,
	},
	GuardUpgrade {
		guard: String,
		/// The new binary.
		binary: Cid,
	},
	GuardTagsInsert {
		guard: String,
		tags: Tags,
	},
	GuardTagsRemove {
		guard: String,
		tags: Tags,
	},
}

impl Reducer for Co {
	type Action = CoAction;

	fn reduce(self, event: &ReducerAction<Self::Action>, context: &mut dyn Context) -> Self {
		let mut result = self;
		reduce(context, &mut result, &event.payload);
		result
	}
}
impl<S: BlockStorage + Clone + 'static> co_api::Guard<S> for Co {
	/// Test if next_head creator is a participant with access.
	async fn verify(
		storage: &S,
		_guard: String,
		state: Cid,
		_heads: BTreeSet<Cid>,
		next_head: Cid,
	) -> Result<bool, anyhow::Error> {
		let next_entry: SignedEntry = storage.get_deserialized(&next_head).await?;
		let participant = next_entry.identity;
		let co: Co = storage.get_deserialized(&state).await?;
		Ok(co
			.participants
			.iter()
			.any(|item| item.1.state.has_access() && item.0 == &participant))
	}
}

fn reduce(context: &mut dyn Context, result: &mut Co, action: &CoAction) {
	match &action {
		CoAction::Create(create) => reduce_create(result, create),
		CoAction::Upgrade { binary, migrate } => reduce_upgrade(result, binary, migrate),
		CoAction::ParticipantInvite { participant, tags } => reduce_participant_invite(result, participant, tags),
		CoAction::ParticipantJoin { participant, tags } => reduce_participant_join(result, participant, tags),
		CoAction::ParticipantPending { participant, tags } => reduce_participant_pending(result, participant, tags),
		CoAction::ParticipantRemove { participant, tags } => reduce_participant_remove(result, participant, tags),
		CoAction::Heads { heads } => reduce_heads(result, heads),
		CoAction::CoreCreate { core, binary, tags } => reduce_core_create(result, core, binary, tags),
		CoAction::CoreRemove { core } => reduce_core_remove(result, core),
		CoAction::ParticipantTagsInsert { participant, tags } => {
			reduce_participant_tags_insert(result, participant, tags)
		},
		CoAction::ParticipantTagsRemove { participant, tags } => {
			reduce_participant_tags_remove(result, participant, tags)
		},
		CoAction::CoreChange { core, state } => reduce_core_change(result, core, state),
		CoAction::CoreUpgrade { core, binary, migrate } => reduce_core_upgrade(result, core, binary, migrate),
		CoAction::CoreTagsInsert { core, tags } => reduce_core_tags_insert(result, core, tags),
		CoAction::CoreTagsRemove { core, tags } => reduce_core_tags_remove(result, core, tags),
		CoAction::TagsInsert { tags } => reduce_tags_insert(result, tags),
		CoAction::TagsRemove { tags } => reduce_tags_remove(result, tags),
		CoAction::NetworkInsert { network } => reduce_network_insert(context, result, network),
		CoAction::NetworkRemove { network } => reduce_network_remove(context, result, network),
		CoAction::GuardCreate { guard, binary, tags } => reduce_guard_create(result, guard, binary, tags),
		CoAction::GuardRemove { guard } => reduce_guard_remove(result, guard),
		CoAction::GuardUpgrade { guard, binary } => reduce_guard_upgrade(result, guard, binary),
		CoAction::GuardTagsInsert { guard, tags } => reduce_guard_tags_insert(result, guard, tags),
		CoAction::GuardTagsRemove { guard, tags } => reduce_guard_tags_remove(result, guard, tags),
	}
}

fn reduce_create(result: &mut Co, create: &CreateAction) {
	// only allowed for empty COs
	// id can not be changed afterwards
	if result.id.as_str().is_empty() {
		result.id = create.id.to_owned();
		result.name = create.name.to_owned();
		result.cores = create.cores.to_owned();
		result.guards = create.guards.to_owned();
		result.participants = create.participants.to_owned();
		result.keys = create
			.key
			.as_ref()
			.map(|key_id| vec![Key { id: key_id.to_owned(), state: KeyState::Active }]);
		result.binary = create.binary;
	}
}

fn reduce_upgrade(result: &mut Co, binary: &Cid, _migrate: &Option<Cid>) {
	result.binary = *binary;
}

fn reduce_participant_invite(result: &mut Co, participant: &String, tags: &Tags) {
	if let Some(item) = result.participants.get_mut(participant) {
		match item.state {
			ParticipantState::Pending | ParticipantState::Inactive => {
				item.state = ParticipantState::Invite;
				item.tags.append(&mut tags.clone());
			},
			_ => {
				// we don't go back from active to invite
			},
		}
	} else {
		result.participants.insert(
			participant.clone(),
			Participant { did: participant.clone(), state: ParticipantState::Invite, tags: tags.clone() },
		);
	}
}

fn reduce_participant_join(result: &mut Co, participant: &String, tags: &Tags) {
	if let Some(participant) = result.participants.get_mut(participant) {
		participant.state = ParticipantState::Active;
		participant.tags.append(&mut tags.clone());
	}
}

fn reduce_participant_pending(result: &mut Co, participant: &String, tags: &Tags) {
	if !result.participants.contains_key(participant) {
		result.participants.insert(
			participant.clone(),
			Participant { did: participant.clone(), state: ParticipantState::Pending, tags: tags.clone() },
		);
	}
}

fn reduce_participant_remove(result: &mut Co, participant: &String, tags: &Tags) {
	let remove = if let Some(item) = result.participants.get_mut(participant) {
		match item.state {
			ParticipantState::Pending => true,
			_ => {
				item.state = ParticipantState::Inactive;
				item.tags.append(&mut tags.clone());
				false
			},
		}
	} else {
		false
	};
	if remove {
		result.participants.remove(participant);
	}
}

fn reduce_heads(result: &mut Co, heads: &BTreeSet<Cid>) {
	result.heads = heads.clone();
}

fn reduce_core_create(result: &mut Co, core: &String, binary: &Cid, tags: &Tags) {
	if !result.cores.contains_key(core) {
		result
			.cores
			.insert(core.clone(), Core { binary: *binary, tags: tags.clone(), state: None });
	}
}

fn reduce_core_remove(result: &mut Co, core: &String) {
	result.cores.remove(core);
}

fn reduce_participant_tags_insert(result: &mut Co, participant: &String, tags: &Tags) {
	if let Some(participant) = result.participants.get_mut(participant) {
		participant.tags.append(&mut tags.clone());
	}
}

fn reduce_participant_tags_remove(result: &mut Co, participant: &String, tags: &Tags) {
	if let Some(participant) = result.participants.get_mut(participant) {
		participant.tags.clear(Some(tags));
	}
}

fn reduce_core_upgrade(result: &mut Co, core: &String, binary: &Cid, _migrate: &Option<Cid>) {
	if let Some(core) = result.cores.get_mut(core) {
		core.binary = *binary;
	}
}

fn reduce_core_change(result: &mut Co, core: &String, state: &Option<Cid>) {
	if let Some(core) = result.cores.get_mut(core) {
		core.state = *state;
	}
}

fn reduce_core_tags_insert(result: &mut Co, core: &String, tags: &Tags) {
	if let Some(core) = result.cores.get_mut(core) {
		core.tags.append(&mut tags.clone());
	}
}

fn reduce_network_remove(context: &mut dyn Context, result: &mut Co, network: &Network) {
	result.network.remove(context.storage_mut(), network);
}

fn reduce_network_insert(context: &mut dyn Context, result: &mut Co, network: &Network) {
	result.network.insert(context.storage_mut(), network.clone());
}

fn reduce_tags_remove(result: &mut Co, tags: &Tags) {
	result.tags.clear(Some(tags));
}

fn reduce_tags_insert(result: &mut Co, tags: &Tags) {
	result.tags.append(&mut tags.clone());
}

fn reduce_core_tags_remove(result: &mut Co, core: &String, tags: &Tags) {
	if let Some(core) = result.cores.get_mut(core) {
		core.tags.clear(Some(tags));
	}
}

fn reduce_guard_create(result: &mut Co, guard_name: &String, binary: &Cid, tags: &Tags) {
	if !result.guards.contains_key(guard_name) {
		result
			.guards
			.insert(guard_name.clone(), Guard { binary: *binary, tags: tags.clone() });
	}
}

fn reduce_guard_remove(result: &mut Co, guard_name: &String) {
	result.guards.remove(guard_name);
}

fn reduce_guard_upgrade(result: &mut Co, guard_name: &String, binary: &Cid) {
	if let Some(guard) = result.guards.get_mut(guard_name) {
		guard.binary = *binary;
	}
}

fn reduce_guard_tags_insert(result: &mut Co, guard_name: &String, tags: &Tags) {
	if let Some(guard) = result.guards.get_mut(guard_name) {
		guard.tags.append(&mut tags.clone());
	}
}

fn reduce_guard_tags_remove(result: &mut Co, guard_name: &String, tags: &Tags) {
	if let Some(guard) = result.guards.get_mut(guard_name) {
		guard.tags.clear(Some(tags));
	}
}
