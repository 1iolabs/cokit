// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use anyhow::anyhow;
use cid::Cid;
use co_api::{
	async_api::Reducer, co, BlockStorage, BlockStorageExt, CoMap, CoMapTransaction, CoSet, CoSetTransaction,
	CoreBlockStorage, Did, Guard, Link, OptionLink, ReducerAction, SignedEntry, WeakCid,
};
use co_core_co::Co;
use futures::{pin_mut, TryStreamExt};
use num_rational::Ratio;
use std::{
	cmp::max,
	collections::{BTreeMap, BTreeSet},
	future::ready,
};

/// Authority actions.
#[co]
pub enum AuthorityAction {
	/// Agree on a specific checkpoint.
	/// The first time the a checkpoint is agreed on marks the start of a consensus round.
	Agree(Checkpoint),

	/// Update consensus settings.
	/// This also requires a majority.
	/// The first time the a AuthorityUpdate is emitted a consensus round starts.
	Update(AuthorityUpdate),
}
impl AuthorityAction {
	pub fn is_same_kind(&self, other: &AuthorityAction) -> bool {
		matches!(
			(self, other),
			(AuthorityAction::Agree(_), AuthorityAction::Agree(_))
				| (AuthorityAction::Update(_), AuthorityAction::Update(_))
		)
	}
}

/// State/Heads Checkpoint.
pub type Checkpoint = (WeakCid, BTreeSet<WeakCid>);

/// Update consensus authority settings.
#[co]
#[derive(Default)]
pub struct AuthorityUpdate {
	/// Update/Insert/Remove authorities.
	///
	/// Insert/Update:
	/// `DID = Some(DidInfo)`
	///
	/// Remove:
	/// `DID = None`
	pub authority: BTreeMap<Did, Option<AuthorityInfo>>,

	/// Update majority.
	pub majority: Option<Option<u32>>,
}

/// Stores additional informations for authorities.
#[co]
pub struct AuthorityInfo {
	/// The authority vote weight.
	pub weight: u32,
}

#[co(state, guard, no_default)]
#[derive(Default)]
pub struct Authority {
	/// The authority.
	#[serde(rename = "a", default, skip_serializing_if = "CoMap::is_empty")]
	pub authority: CoMap<Did, AuthorityInfo>,

	/// The majority required to reach consensus.
	///
	/// If not specified `2/3` of vote weights are assumed:
	/// $W := \sum_{a \in A} w_a$
	/// $M := \lceil \text{W} \times \frac{2}{3} \rceil$
	#[serde(rename = "m", default, skip_serializing_if = "Option::is_none")]
	pub majority: Option<u32>,

	/// Latest reached consensus.
	#[serde(rename = "c")]
	pub consensus: Option<Checkpoint>,

	/// Pending consensus actions.
	#[serde(rename = "p", default, skip_serializing_if = "CoSet::is_empty")]
	pub pending: CoSet<Link<ReducerAction<AuthorityAction>>>,

	/// Total count of reached consensus.
	#[serde(rename = "r")]
	pub consensus_count: u64,

	/// Total count of reached update consensus.
	#[serde(rename = "n")]
	pub update_count: u64,
}
impl Reducer<AuthorityAction> for Authority {
	async fn reduce(
		state_link: OptionLink<Self>,
		event_link: Link<ReducerAction<AuthorityAction>>,
		storage: &CoreBlockStorage,
	) -> Result<Link<Self>, anyhow::Error> {
		// get
		let event = storage.get_value(&event_link).await?;
		let mut state = storage.get_value_or_default(&state_link).await?;

		// open
		let mut authority_changed = false;
		let mut authority = state.authority.open(storage).await?;
		let mut pending = state.pending.open(storage).await?;

		// get majority
		let majority = get_majority(&authority, state.majority).await?;

		// check majority
		if is_majority(storage, &authority, &pending, &event, majority).await? {
			// clear pending
			//  we remove all of same kind to clear out previous unfinalized rounds
			let remove = pending
				.stream()
				.try_filter_map({
					|link| {
						let action = event.payload.clone();
						async move {
							let item = storage.get_value(&link).await?;
							if item.payload.is_same_kind(&action) {
								Ok(Some(link))
							} else {
								Ok(None)
							}
						}
					}
				})
				.try_collect::<Vec<_>>()
				.await?;
			for link in remove {
				pending.remove(link).await?;
			}

			// apply
			match event.payload {
				AuthorityAction::Agree(consensus) => {
					state.consensus = Some(consensus);
					state.consensus_count += 1;
				},
				AuthorityAction::Update(update) => {
					if let Some(majority) = update.majority {
						state.majority = majority;
					}
					for (did, did_change) in update.authority {
						authority_changed = true;
						match did_change {
							Some(info) => {
								authority.insert(did, info).await?;
							},
							None => {
								authority.remove(did).await?;
							},
						}
					}
					state.update_count += 1;
				},
			}
		} else {
			// set as pending as we have no majority yet
			pending.insert(event_link).await?;
		}

		// store
		if authority_changed {
			state.authority = authority.store().await?;
		}
		state.pending = pending.store().await?;
		Ok(storage.set_value(&state).await?)
	}
}
impl Guard for Authority {
	async fn verify(
		storage: &CoreBlockStorage,
		guard: String,
		state: Cid,
		_heads: BTreeSet<Cid>,
		next_head: Cid,
	) -> Result<bool, anyhow::Error> {
		let next_entry: SignedEntry = storage.get_deserialized(&next_head).await?;
		let co: Co = storage.get_deserialized(&state).await?;

		// find co-core-poa core name
		let guard = co.guards.get(&guard).ok_or(anyhow!("Guard not found: {}", guard))?;
		let core_name = guard.tags.string("core").unwrap_or("poa");

		// core
		let core = co.cores.get(core_name).ok_or(anyhow!("Core not found: {}", core_name))?;
		if let Some(state) = &core.state {
			let authority: Authority = storage.get_deserialized(state).await?;
			if let Some((_consensus_state, consensus_heads)) = authority.consensus {
				// find the max consensus time (basically the log height without conflicts)
				let mut consensus_time = 0;
				for consensus_head in &consensus_heads {
					let consensus_head_entry: SignedEntry = storage.get_deserialized(consensus_head.as_ref()).await?;
					consensus_time = max(consensus_time, consensus_head_entry.entry.clock.time);
				}

				// compare if next head log time if after it
				Ok(next_entry.entry.clock.time > consensus_time)
			} else {
				// no consensus yet
				Ok(true)
			}
		} else {
			// no votes and consensus yet
			Ok(true)
		}
	}
}

/// Get majority.
async fn get_majority<S>(
	authority: &CoMapTransaction<S, Did, AuthorityInfo>,
	majority: Option<u32>,
) -> Result<u32, anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	match majority {
		Some(majority) => Ok(majority),
		None => {
			let all_weights = authority
				.stream()
				.try_fold(0, |acc, (_did, info)| ready(Ok(acc + info.weight)))
				.await?;
			let ratio = Ratio::<u32>::new(2, 3);
			Ok((ratio * all_weights).ceil().to_integer())
		},
	}
}

/// Whether `pending + action` represent the `majority`.
async fn is_majority<S: BlockStorage + Clone + 'static>(
	storage: &S,
	authority: &CoMapTransaction<S, Did, AuthorityInfo>,
	pending: &CoSetTransaction<S, Link<ReducerAction<AuthorityAction>>>,
	action: &ReducerAction<AuthorityAction>,
	majority: u32,
) -> Result<bool, anyhow::Error> {
	let pending_stream = pending.stream();
	pin_mut!(pending_stream);
	let mut weights = 0;

	// action weight
	if let Some(info) = authority.get(&action.from).await? {
		weights += info.weight;
		if weights >= majority {
			return Ok(true);
		}
	}

	// pending weights
	while let Some(pending) = pending_stream.try_next().await? {
		let pending = storage.get_value(&pending).await?;
		if pending.payload == action.payload {
			if let Some(info) = authority.get(&pending.from).await? {
				weights += info.weight;
				if weights >= majority {
					return Ok(true);
				}
			}
		}
	}

	Ok(false)
}
