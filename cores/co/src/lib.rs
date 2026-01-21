use cid::Cid;
use co_api::{
	async_api::Reducer, co, BlockStorage, BlockStorageExt, CoId, CoMap, CoSet, CoreBlockStorage, Did, Link, Network,
	OptionLink, ReducerAction, SignedEntry, StorageError, Tags,
};
use serde::de::IgnoredAny;
use std::collections::{BTreeMap, BTreeSet};

#[co(state, guard, no_default)]
#[non_exhaustive]
pub struct Co {
	/// CO UUID.
	pub id: CoId,

	/// CO Tags.
	#[serde(rename = "t", default, skip_serializing_if = "Tags::is_empty")]
	pub tags: Tags,

	/// CO Name.
	#[serde(rename = "n")]
	pub name: String,

	/// CO Core Binary.
	#[serde(rename = "b")]
	pub binary: Cid,

	/// CO Participants.
	#[serde(rename = "p", default, skip_serializing_if = "CoMap::is_empty")]
	pub participants: CoMap<Did, Participant>,

	/// CO Streams with the associated state reference.
	///
	/// Key: Core Instance
	#[serde(rename = "c")]
	pub cores: BTreeMap<String, Core>,

	/// Co Guards.
	#[serde(rename = "g", default, skip_serializing_if = "BTreeMap::is_empty")]
	pub guards: BTreeMap<String, Guard>,

	/// CO Encryption Keys.
	/// The first (index: 0) key is the active key.
	/// Keys are normally stored in the Local CO.
	#[serde(rename = "k", default, skip_serializing_if = "Option::is_none")]
	pub keys: Option<Vec<Key>>,

	/// CO network services.
	/// See: [`libp2p::PeerId`]
	// #[co_api::Dag]
	#[serde(rename = "s", default, skip_serializing_if = "CoSet::is_empty")]
	pub network: CoSet<Network>,
}
impl Default for Co {
	fn default() -> Self {
		Self {
			id: "".into(),
			tags: Default::default(),
			name: Default::default(),
			binary: Default::default(),
			participants: Default::default(),
			cores: Default::default(),
			keys: Default::default(),
			network: Default::default(),
			guards: Default::default(),
		}
	}
}
impl Reducer<CoAction> for Co {
	async fn reduce(
		state: OptionLink<Self>,
		event: Link<ReducerAction<CoAction>>,
		storage: &CoreBlockStorage,
	) -> Result<Link<Self>, anyhow::Error> {
		let mut result = storage.get_value_or_default(&state).await?;
		let event = storage.get_value(&event).await?;
		reduce(storage, &mut result, &event.payload).await?;
		let state = storage.set_value(&result).await?;
		Ok(state)
	}
}
impl co_api::Guard for Co {
	/// Test:
	/// - the specified core exists.
	/// - if next_head creator is a participant with access.
	async fn verify(
		storage: &CoreBlockStorage,
		_guard: String,
		state: Cid,
		_heads: BTreeSet<Cid>,
		next_head: Cid,
	) -> Result<bool, anyhow::Error> {
		let next_entry: SignedEntry = storage.get_deserialized(&next_head).await?;
		let co: Co = storage.get_deserialized(&state).await?;

		// participant
		let participant = next_entry.identity;
		let has_access = if let Some(participant) = co.participants.get(storage, &participant).await? {
			participant.state.has_access()
		} else {
			false
		};
		if !has_access {
			return Ok(false);
		}

		// core
		let action: ReducerAction<IgnoredAny> = storage.get_deserialized(&next_entry.entry.payload).await?;
		let has_core = action.core == "co" || co.cores.contains_key(&action.core);
		if !has_core {
			return Ok(false);
		}

		// ok
		Ok(true)
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
impl Core {
	pub fn with_state(mut self, state: Option<Cid>) -> Self {
		self.state = state;
		self
	}
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
		matches!(self, ParticipantState::Active)
	}

	pub fn has_access(&self) -> bool {
		matches!(self, ParticipantState::Active | ParticipantState::Invite)
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

	pub fn with_guard(mut self, guard_name: String, guard: Guard) -> Self {
		self.guards.insert(guard_name, guard);
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

/// Reduce [`CoAction`] to result [`Co`] state.
/// Returns [`true`] if anything in result has changed.
async fn reduce<S>(storage: &S, result: &mut Co, action: &CoAction) -> Result<bool, anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	Ok(match &action {
		CoAction::Create(create) => reduce_create(storage, result, create).await?,
		CoAction::Upgrade { binary, migrate } => reduce_upgrade(result, binary, migrate),
		CoAction::ParticipantInvite { participant, tags } => {
			reduce_participant_invite(storage, result, participant, tags).await?
		},
		CoAction::ParticipantJoin { participant, tags } => {
			reduce_participant_join(storage, result, participant, tags).await?
		},
		CoAction::ParticipantPending { participant, tags } => {
			reduce_participant_pending(storage, result, participant, tags).await?
		},
		CoAction::ParticipantRemove { participant, tags } => {
			reduce_participant_remove(storage, result, participant, tags).await?
		},
		CoAction::CoreCreate { core, binary, tags } => reduce_core_create(result, core, binary, tags),
		CoAction::CoreRemove { core } => reduce_core_remove(result, core),
		CoAction::ParticipantTagsInsert { participant, tags } => {
			reduce_participant_tags_insert(storage, result, participant, tags).await?
		},
		CoAction::ParticipantTagsRemove { participant, tags } => {
			reduce_participant_tags_remove(storage, result, participant, tags).await?
		},
		CoAction::CoreChange { core, state } => reduce_core_change(result, core, state),
		CoAction::CoreUpgrade { core, binary, migrate } => reduce_core_upgrade(result, core, binary, migrate),
		CoAction::CoreTagsInsert { core, tags } => reduce_core_tags_insert(result, core, tags),
		CoAction::CoreTagsRemove { core, tags } => reduce_core_tags_remove(result, core, tags),
		CoAction::TagsInsert { tags } => reduce_tags_insert(result, tags),
		CoAction::TagsRemove { tags } => reduce_tags_remove(result, tags),
		CoAction::NetworkInsert { network } => reduce_network_insert(storage, result, network).await?,
		CoAction::NetworkRemove { network } => reduce_network_remove(storage, result, network).await?,
		CoAction::GuardCreate { guard, binary, tags } => reduce_guard_create(result, guard, binary, tags),
		CoAction::GuardRemove { guard } => reduce_guard_remove(result, guard),
		CoAction::GuardUpgrade { guard, binary } => reduce_guard_upgrade(result, guard, binary),
		CoAction::GuardTagsInsert { guard, tags } => reduce_guard_tags_insert(result, guard, tags),
		CoAction::GuardTagsRemove { guard, tags } => reduce_guard_tags_remove(result, guard, tags),
	})
}

async fn reduce_create<S>(storage: &S, result: &mut Co, create: &CreateAction) -> Result<bool, anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	// only allowed for empty COs
	// id can not be changed afterwards
	if !result.id.as_str().is_empty() {
		return Err(anyhow::anyhow!("Create is only supported once."));
	}

	// apply
	result.id = create.id.to_owned();
	result.name = create.name.to_owned();
	result.cores = create.cores.to_owned();
	result.guards = create.guards.to_owned();
	result.participants = CoMap::from_iter(storage, create.participants.clone()).await?;
	result.keys = create
		.key
		.as_ref()
		.map(|key_id| vec![Key { id: key_id.to_owned(), state: KeyState::Active }]);
	result.binary = create.binary;
	Ok(true)
}

fn reduce_upgrade(result: &mut Co, binary: &Cid, _migrate: &Option<Cid>) -> bool {
	result.binary = *binary;
	true
}

async fn reduce_participant_invite<S>(
	storage: &S,
	result: &mut Co,
	participant: &String,
	tags: &Tags,
) -> Result<bool, anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut participants = result.participants.open(storage).await?;
	if let Some(mut item) = participants.get(participant).await? {
		match item.state {
			ParticipantState::Pending | ParticipantState::Inactive => {
				item.state = ParticipantState::Invite;
				item.tags.append(&mut tags.clone());
				participants.insert(participant.clone(), item).await?;
			},
			_ => {
				// we don't go back from active to invite
			},
		}
	} else {
		participants
			.insert(
				participant.clone(),
				Participant { did: participant.clone(), state: ParticipantState::Invite, tags: tags.clone() },
			)
			.await?;
	};
	let next_participants = participants.store().await?;
	Ok(if result.participants != next_participants {
		result.participants = next_participants;
		true
	} else {
		false
	})
}

async fn reduce_participant_join<S>(
	storage: &S,
	result: &mut Co,
	participant: &String,
	tags: &Tags,
) -> Result<bool, anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut participants = result.participants.open(storage).await?;
	Ok(if let Some(mut item) = participants.get(participant).await? {
		item.state = ParticipantState::Active;
		item.tags.append(&mut tags.clone());
		participants.insert(participant.clone(), item).await?;
		result.participants = participants.store().await?;
		true
	} else {
		false
	})
}

async fn reduce_participant_pending<S>(
	storage: &S,
	result: &mut Co,
	participant: &String,
	tags: &Tags,
) -> Result<bool, anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut participants = result.participants.open(storage).await?;
	Ok(if !participants.contains_key(participant).await? {
		participants
			.insert(
				participant.clone(),
				Participant { did: participant.clone(), state: ParticipantState::Pending, tags: tags.clone() },
			)
			.await?;
		result.participants = participants.store().await?;
		true
	} else {
		false
	})
}

async fn reduce_participant_remove<S>(
	storage: &S,
	result: &mut Co,
	participant: &String,
	tags: &Tags,
) -> Result<bool, anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut participants = result.participants.open(storage).await?;
	let remove = if let Some(mut item) = participants.get(participant).await? {
		match item.state {
			ParticipantState::Pending => true,
			_ => {
				item.state = ParticipantState::Inactive;
				item.tags.append(&mut tags.clone());
				participants.insert(participant.clone(), item).await?;
				result.participants = participants.store().await?;
				false
			},
		}
	} else {
		false
	};
	Ok(if remove {
		participants.remove(participant.clone()).await?;
		result.participants = participants.store().await?;
		true
	} else {
		false
	})
}

fn reduce_core_create(result: &mut Co, core: &String, binary: &Cid, tags: &Tags) -> bool {
	if !result.cores.contains_key(core) {
		result
			.cores
			.insert(core.clone(), Core { binary: *binary, tags: tags.clone(), state: None });
		true
	} else {
		false
	}
}

fn reduce_core_remove(result: &mut Co, core: &String) -> bool {
	result.cores.remove(core).is_some()
}

async fn reduce_participant_tags_insert<S>(
	storage: &S,
	result: &mut Co,
	participant: &String,
	tags: &Tags,
) -> Result<bool, anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut participants = result.participants.open(storage).await?;
	Ok(if let Some(mut item) = participants.get(participant).await? {
		item.tags.append(&mut tags.clone());
		participants.insert(participant.clone(), item).await?;
		result.participants = participants.store().await?;
		true
	} else {
		false
	})
}

async fn reduce_participant_tags_remove<S>(
	storage: &S,
	result: &mut Co,
	participant: &String,
	tags: &Tags,
) -> Result<bool, anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut participants = result.participants.open(storage).await?;
	Ok(if let Some(mut item) = participants.get(participant).await? {
		item.tags.clear(Some(tags));
		participants.insert(participant.clone(), item).await?;
		result.participants = participants.store().await?;
		true
	} else {
		false
	})
}

fn reduce_core_upgrade(result: &mut Co, core: &String, binary: &Cid, _migrate: &Option<Cid>) -> bool {
	if let Some(core) = result.cores.get_mut(core) {
		core.binary = *binary;
		true
	} else {
		false
	}
}

fn reduce_core_change(result: &mut Co, core: &String, state: &Option<Cid>) -> bool {
	if let Some(core) = result.cores.get_mut(core) {
		let result = core.state != *state;
		core.state = *state;
		result
	} else {
		false
	}
}

fn reduce_core_tags_insert(result: &mut Co, core: &String, tags: &Tags) -> bool {
	if let Some(core) = result.cores.get_mut(core) {
		core.tags.append(&mut tags.clone());
		true
	} else {
		false
	}
}

async fn reduce_network_remove<S>(storage: &S, result: &mut Co, network: &Network) -> Result<bool, StorageError>
where
	S: BlockStorage + Clone + 'static,
{
	result.network.remove(storage, network.clone()).await
}

async fn reduce_network_insert<S>(storage: &S, result: &mut Co, network: &Network) -> Result<bool, StorageError>
where
	S: BlockStorage + Clone + 'static,
{
	result.network.insert(storage, network.clone()).await?;
	Ok(true)
}

fn reduce_tags_remove(result: &mut Co, tags: &Tags) -> bool {
	result.tags.clear(Some(tags));
	true
}

fn reduce_tags_insert(result: &mut Co, tags: &Tags) -> bool {
	result.tags.append(&mut tags.clone());
	true
}

fn reduce_core_tags_remove(result: &mut Co, core: &String, tags: &Tags) -> bool {
	if let Some(core) = result.cores.get_mut(core) {
		core.tags.clear(Some(tags));
		true
	} else {
		false
	}
}

fn reduce_guard_create(result: &mut Co, guard_name: &String, binary: &Cid, tags: &Tags) -> bool {
	if !result.guards.contains_key(guard_name) {
		result
			.guards
			.insert(guard_name.clone(), Guard { binary: *binary, tags: tags.clone() });
		true
	} else {
		false
	}
}

fn reduce_guard_remove(result: &mut Co, guard_name: &String) -> bool {
	result.guards.remove(guard_name).is_some()
}

fn reduce_guard_upgrade(result: &mut Co, guard_name: &String, binary: &Cid) -> bool {
	if let Some(guard) = result.guards.get_mut(guard_name) {
		let result = guard.binary != *binary;
		guard.binary = *binary;
		result
	} else {
		false
	}
}

fn reduce_guard_tags_insert(result: &mut Co, guard_name: &String, tags: &Tags) -> bool {
	if let Some(guard) = result.guards.get_mut(guard_name) {
		guard.tags.append(&mut tags.clone());
		true
	} else {
		false
	}
}

fn reduce_guard_tags_remove(result: &mut Co, guard_name: &String, tags: &Tags) -> bool {
	if let Some(guard) = result.guards.get_mut(guard_name) {
		guard.tags.clear(Some(tags))
	} else {
		false
	}
}
