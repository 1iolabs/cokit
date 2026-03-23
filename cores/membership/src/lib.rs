// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_api::{
	async_api::Reducer, co, BlockStorageExt, CoId, CoMap, CoReference, CoreBlockStorage, Did, IsDefault, Link,
	OptionLink, ReducerAction, StorageError, Tags, WeakCid,
};
use futures::{FutureExt, TryStreamExt};
use std::collections::{BTreeMap, BTreeSet};

/// Membership COre.
/// Stores membership information of an CO (counterpart to co participants).
#[co(state)]
pub struct Memberships {
	pub memberships: CoMap<CoId, Membership>,
}

/// Membership entry.
#[co]
pub struct Membership {
	/// The CO Unique Identifier.
	pub id: CoId,

	/// The membership states per DID.
	pub did: BTreeMap<Did, MembershipState>,

	/// CO States. This can be multiple states if we have heads that are not joined yet.
	pub state: BTreeSet<CoState>,

	/// Some encryption key URI if the CO is encrypted.
	pub key: Option<String>,

	/// Membership tags.
	pub tags: Tags,
}

/// A CO State entry.
/// Contains heads the computed state for the heads and an option encryption mapping.
#[co]
pub struct CoState {
	/// The CO root state (usually co-core-co) and heads.
	/// Note: This is not an Option so we can not be member of an emtpy CO (which has no id anyway).
	/// Note: We want to use `CoReference::Weak` instead of `WeakCid` here because we need to have mappings generated
	/// for it.
	pub state: Link<CoReference<(Cid, BTreeSet<Cid>)>>,

	// TODO mark as external as this field shouldn't be further resolved when pinning
	// TODO https://gitlab.1io.com/1io/co-sdk/-/issues/47
	/// The encryption mapping if the CO is encrypted.
	#[serde(skip_serializing_if = "Option::is_none", default)]
	pub encryption_mapping: Option<Cid>,
}

/// Membership state.
///
/// # Guarantees
/// - Sortable from active (low) to inactive (high).
#[co(repr)]
#[non_exhaustive]
#[repr(u8)]
pub enum MembershipState {
	/// Active membership.
	Active = 10,

	/// Pending state resolution.
	/// Has CoInviteMetadata to connect, but needs to resolve CO state
	/// (and optionally encryption key) from network before use.
	///
	/// Related membership Tags:
	///  `co-invite-metadata: CoInviteMetadata`
	Pending = 15,

	/// Pending join by us.
	/// The goal of a join is to end up as a CO's participant.
	/// We are signaling that we want to be an participant in their CO.
	///
	/// Use Cases:
	/// - This is a pending join triggered by an invite waiting for completion.
	/// - This is waiting for CO participant acceptation/rejection (remote).
	///
	/// Related membership Tags:
	///  `co-invite: CoInviteMetadata`
	///  `join-date: Date`
	Join = 20,

	/// Pending invite by some participant of the CO.
	/// The goal of a invite is to end up as a CO's participant.
	/// A CO's participant are signaling that they want us to be an participant in their CO.
	///
	/// Use Cases:
	/// - This is waiting for our acceptation/rejection.
	/// - Accept invite by change membership state to [`MembershipState::Join`].
	/// - Reject invite by removing the membership using [`MembershipsAction::Remove`].
	///
	/// Related membership Tags:
	///  `co-invite: CoInviteMetadata`
	Invite = 30,

	/// Inactive membership.
	Inactive = 40,
}

/// Membership options
#[co]
#[derive(Default)]
#[non_exhaustive]
pub struct MembershipOptions {
	/// Insert state.
	pub state: Option<BTreeSet<CoState>>,

	/// Update key.
	pub key: Option<String>,

	/// Insert tags.
	pub tags: Option<Tags>,
}
impl MembershipOptions {
	pub fn with_added_state(mut self, co_state: CoState) -> Self {
		self.state = {
			let mut state = self.state.unwrap_or_default();
			state.insert(co_state);
			Some(state)
		};
		self
	}

	pub fn with_key(mut self, key: String) -> Self {
		self.key = Some(key);
		self
	}

	pub fn with_tags(mut self, tags: Tags) -> Self {
		self.tags = Some(tags);
		self
	}
}

#[co]
pub enum MembershipsAction {
	/// Active membership.
	///
	/// # Use Cases
	/// - CO Creation
	/// - Direct join
	/// - Activation of [`MembershipState::Pending`] or [`MembershipState::Join`].
	///
	/// # Membership State
	/// - [`MembershipState::Active`]
	Join {
		id: CoId,
		did: Did,
		#[serde(default, skip_serializing_if = "IsDefault::is_default")]
		options: MembershipOptions,
	},

	/// Received invite, awaiting user acceptance.
	///
	/// # Next Actions
	/// - `MembershipsAction::InviteAccept`
	/// - `MembershipsAction::Remove`
	///
	/// # Membership State
	/// - [`MembershipState::Invite`]
	Invited {
		id: CoId,
		did: Did,
		#[serde(default, skip_serializing_if = "IsDefault::is_default")]
		options: MembershipOptions,
	},

	/// Request to join a CO.
	///
	/// # Use Case
	/// - We requested to join a CO and this request is waiting for CO participant acceptation/rejection (remote).
	///
	/// # Membership State
	/// - [`MembershipState::Join`]
	JoinRequest {
		id: CoId,
		did: Did,
		#[serde(default, skip_serializing_if = "IsDefault::is_default")]
		options: MembershipOptions,
	},

	/// Pending state resolution — has metadata, needs to fetch CO state from network.
	///
	/// # Next Actions
	/// - `MembershipsAction::Join`
	///
	/// # Membership State
	/// - [`MembershipState::Pending`]
	JoinPending {
		id: CoId,
		did: Did,
		#[serde(default, skip_serializing_if = "IsDefault::is_default")]
		options: MembershipOptions,
	},

	/// User accepts invite.
	///
	/// # Next Actions
	/// - `MembershipsAction::Join`
	///
	/// # Membership State
	/// - [`MembershipState::Join`]
	InviteAccept {
		id: CoId,
		did: Did,
		#[serde(default, skip_serializing_if = "IsDefault::is_default")]
		options: MembershipOptions,
	},

	/// Deactivate Membership.
	///
	/// # Membership State
	/// - [`MembershipState::Inactive`]
	Deactivate { id: CoId, did: Did },

	/// Update state of a CO.
	Update {
		id: CoId,
		state: CoState,
		/// Remove all [`CoState`] which heads are fully covered.
		remove: BTreeSet<WeakCid>,
	},

	/// Change the active encryption key reference which is used the read the current heads/state.
	ChangeKey {
		/// The CO.
		id: CoId,

		/// New key URI.
		key: String,
	},

	/// Insert tags for membership.
	TagsInsert { id: CoId, tags: Tags },

	/// Remove tags for membership.
	TagsRemove { id: CoId, tags: Tags },

	/// Remove CO membership.
	Remove {
		/// The CO.
		id: CoId,

		/// The identity to remove.
		/// If not specified all memberships for a CO are removed.
		did: Option<Did>,
	},
}

impl Reducer<MembershipsAction> for Memberships {
	async fn reduce(
		state_ref: OptionLink<Self>,
		action_ref: Link<ReducerAction<MembershipsAction>>,
		storage: &CoreBlockStorage,
	) -> Result<Link<Self>, anyhow::Error> {
		let action = storage.get_value(&action_ref).await?;
		let mut result = storage.get_value_or_default(&state_ref).await?;
		match &action.payload {
			MembershipsAction::Join { id, did, options } => {
				reduce_join(&mut result.memberships, storage, id, did, options).boxed().await?;
			},
			MembershipsAction::Invited { id, did, options } => {
				reduce_invited(&mut result.memberships, storage, id, did, options)
					.boxed()
					.await?;
			},
			MembershipsAction::JoinRequest { id, did, options } => {
				reduce_join_request(&mut result.memberships, storage, id, did, options)
					.boxed()
					.await?;
			},
			MembershipsAction::JoinPending { id, did, options } => {
				reduce_join_pending(&mut result.memberships, storage, id, did, options)
					.boxed()
					.await?;
			},
			MembershipsAction::InviteAccept { id, did, options } => {
				reduce_invite_accept(&mut result.memberships, storage, id, did, options)
					.boxed()
					.await?;
			},
			MembershipsAction::Deactivate { id, did } => {
				reduce_deactivate(&mut result.memberships, storage, id, did).boxed().await?;
			},
			MembershipsAction::Update { id, state, remove } => {
				reduce_update(&mut result.memberships, storage, id, state, remove)
					.boxed()
					.await?;
			},
			MembershipsAction::ChangeKey { id, key } => {
				reduce_change_key(&mut result.memberships, storage, id, key).boxed().await?;
			},
			MembershipsAction::TagsInsert { id, tags } => {
				reduce_tags_insert(&mut result.memberships, storage, id, tags).boxed().await?;
			},
			MembershipsAction::TagsRemove { id, tags } => {
				reduce_tags_remove(&mut result.memberships, storage, id, tags).boxed().await?;
			},
			MembershipsAction::Remove { id, did } => {
				reduce_remove(&mut result.memberships, storage, id, did).boxed().await?;
			},
		}
		Ok(storage.set_value(&result).await?)
	}
}

async fn reduce_join(
	memberships: &mut CoMap<CoId, Membership>,
	storage: &CoreBlockStorage,
	id: &CoId,
	did: &Did,
	options: &MembershipOptions,
) -> Result<(), anyhow::Error> {
	if let Some(existing) = memberships.get(storage, id).await? {
		match existing.did.get(did) {
			None => {
				let did = did.clone();
				let options = options.clone();
				memberships
					.update(storage, id.clone(), move |m| {
						m.did.insert(did, MembershipState::Active);
						apply_options(m, options);
					})
					.await?;
			},
			Some(MembershipState::Pending | MembershipState::Join) => {
				let did = did.clone();
				let options = options.clone();
				memberships
					.update(storage, id.clone(), move |m| {
						if let Some(ms) = m.did.get_mut(&did) {
							*ms = MembershipState::Active;
						}
						apply_options(m, options);
					})
					.await?;
			},
			Some(MembershipState::Active) => {},
			Some(membership_state) => {
				anyhow::bail!("cannot activate membership for {id} with did {did}: state is {membership_state:?}")
			},
		}
	} else {
		insert_membership(memberships, storage, id.clone(), did.clone(), MembershipState::Active, options.clone())
			.await?;
	}
	Ok(())
}

async fn reduce_invited(
	memberships: &mut CoMap<CoId, Membership>,
	storage: &CoreBlockStorage,
	id: &CoId,
	did: &Did,
	options: &MembershipOptions,
) -> Result<(), anyhow::Error> {
	insert_did(memberships, storage, id.clone(), did.clone(), MembershipState::Invite, options.clone()).await
}

async fn reduce_join_request(
	memberships: &mut CoMap<CoId, Membership>,
	storage: &CoreBlockStorage,
	id: &CoId,
	did: &Did,
	options: &MembershipOptions,
) -> Result<(), anyhow::Error> {
	insert_did(memberships, storage, id.clone(), did.clone(), MembershipState::Join, options.clone()).await
}

async fn reduce_join_pending(
	memberships: &mut CoMap<CoId, Membership>,
	storage: &CoreBlockStorage,
	id: &CoId,
	did: &Did,
	options: &MembershipOptions,
) -> Result<(), anyhow::Error> {
	insert_did(memberships, storage, id.clone(), did.clone(), MembershipState::Pending, options.clone()).await
}

async fn reduce_invite_accept(
	memberships: &mut CoMap<CoId, Membership>,
	storage: &CoreBlockStorage,
	id: &CoId,
	did: &Did,
	options: &MembershipOptions,
) -> Result<(), anyhow::Error> {
	if let Some(membership) = memberships.get(storage, id).await? {
		if let Some(&membership_state) = membership.did.get(did) {
			anyhow::ensure!(
				membership_state == MembershipState::Invite,
				"cannot accept invite for {id} with did {did}: state is {membership_state:?}"
			);
		}
		let did = did.clone();
		let options = options.clone();
		memberships
			.update(storage, id.clone(), move |membership| {
				// insert or transition to Join
				membership.did.insert(did, MembershipState::Join);
				apply_options(membership, options);
			})
			.await?;
	} else {
		// no membership yet (auto-accept) - create with Join state
		insert_membership(memberships, storage, id.clone(), did.clone(), MembershipState::Join, options.clone())
			.await?;
	}
	Ok(())
}

async fn reduce_deactivate(
	memberships: &mut CoMap<CoId, Membership>,
	storage: &CoreBlockStorage,
	id: &CoId,
	did: &Did,
) -> Result<(), anyhow::Error> {
	let did = did.clone();
	memberships
		.update(storage, id.clone(), move |membership| {
			if let Some(membership_state) = membership.did.get_mut(&did) {
				*membership_state = MembershipState::Inactive;
			}
		})
		.await?;
	Ok(())
}

async fn reduce_update(
	memberships: &mut CoMap<CoId, Membership>,
	storage: &CoreBlockStorage,
	id: &CoId,
	state: &CoState,
	remove: &BTreeSet<WeakCid>,
) -> Result<(), anyhow::Error> {
	memberships
		.try_update_async(storage, id.clone(), {
			let remove = remove.iter().map(WeakCid::cid).collect::<BTreeSet<Cid>>();
			let state = state.clone();
			let storage = storage.clone();
			move |mut membership| async move {
				membership.state = filter_state(storage, &membership.state, &remove).await?;
				membership.state.insert(state);
				Ok(membership)
			}
		})
		.await?;
	Ok(())
}

async fn reduce_change_key(
	memberships: &mut CoMap<CoId, Membership>,
	storage: &CoreBlockStorage,
	id: &CoId,
	key: &str,
) -> Result<(), anyhow::Error> {
	let key = key.to_owned();
	memberships
		.update(storage, id.clone(), move |membership| {
			membership.key = Some(key);
		})
		.await?;
	Ok(())
}

async fn reduce_tags_insert(
	memberships: &mut CoMap<CoId, Membership>,
	storage: &CoreBlockStorage,
	id: &CoId,
	tags: &Tags,
) -> Result<(), anyhow::Error> {
	let mut tags = tags.clone();
	memberships
		.update(storage, id.clone(), move |membership| {
			membership.tags.append(&mut tags);
		})
		.await?;
	Ok(())
}

async fn reduce_tags_remove(
	memberships: &mut CoMap<CoId, Membership>,
	storage: &CoreBlockStorage,
	id: &CoId,
	tags: &Tags,
) -> Result<(), anyhow::Error> {
	let tags = tags.clone();
	memberships
		.update(storage, id.clone(), move |membership| {
			membership.tags.clear(Some(&tags));
		})
		.await?;
	Ok(())
}

async fn reduce_remove(
	memberships: &mut CoMap<CoId, Membership>,
	storage: &CoreBlockStorage,
	id: &CoId,
	did: &Option<Did>,
) -> Result<(), anyhow::Error> {
	match did {
		None => {
			memberships.remove(storage, id.clone()).await?;
		},
		Some(did) => {
			let did = did.clone();
			memberships
				.update(storage, id.clone(), move |membership| {
					membership.did.remove(&did);
				})
				.await?;
		},
	}
	Ok(())
}

/// Add a DID to a membership. Errors if CoId+Did already exists.
async fn insert_did(
	memberships: &mut CoMap<CoId, Membership>,
	storage: &CoreBlockStorage,
	id: CoId,
	did: Did,
	membership_state: MembershipState,
	options: MembershipOptions,
) -> Result<(), anyhow::Error> {
	if let Some(existing) = memberships.get(storage, &id).await? {
		anyhow::ensure!(!existing.did.contains_key(&did), "membership already exists for {id} with did {did}");
		memberships
			.update(storage, id, move |membership| {
				membership.did.insert(did, membership_state);
				apply_options(membership, options);
			})
			.await?;
	} else {
		insert_membership(memberships, storage, id, did, membership_state, options).await?;
	}
	Ok(())
}

/// Insert a new membership entry.
async fn insert_membership(
	memberships: &mut CoMap<CoId, Membership>,
	storage: &CoreBlockStorage,
	id: CoId,
	did: Did,
	membership_state: MembershipState,
	options: MembershipOptions,
) -> Result<(), anyhow::Error> {
	memberships
		.insert(
			storage,
			id.clone(),
			Membership {
				id,
				did: BTreeMap::from([(did, membership_state)]),
				state: options.state.unwrap_or_default(),
				key: options.key,
				tags: options.tags.unwrap_or_default(),
			},
		)
		.await?;
	Ok(())
}

/// Apply optional state/key/tags to a membership.
fn apply_options(membership: &mut Membership, options: MembershipOptions) {
	if let Some(state) = options.state {
		membership.state.extend(state);
	}
	if let Some(key) = options.key {
		membership.key = Some(key);
	}
	if let Some(mut tags) = options.tags {
		membership.tags.append(&mut tags);
	}
}

async fn filter_state(
	storage: CoreBlockStorage,
	state: &BTreeSet<CoState>,
	remove: &BTreeSet<Cid>,
) -> Result<BTreeSet<CoState>, StorageError> {
	async_stream::try_stream! {
		for item in state {
			let (_state, heads) = storage.get_value(&item.state).await?.into_value();
			if !remove.is_superset(&heads) {
				yield item.clone();
			}
		}
	}
	.try_collect()
	.await
}
