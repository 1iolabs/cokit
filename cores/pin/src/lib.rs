// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_api::{co, BlockStorageExt, CoMap, CoSet, CoreBlockStorage, Link, OptionLink, Reducer, ReducerAction, Tags};
use futures::TryStreamExt;

/// COre that handles pinning and unpinning
#[co(state)]
pub struct Pin {
	/// Map of pinned content ids. Keyed by the referenced content ids. The value is a set
	/// of tags making pinning and unpinning idempotent. A service may use tags with unique identifiers to ensure
	/// that a cid stays pinned as it then cannot be unpinned by other services. To unpin, all tags must match.
	pub pins: CoMap<Cid, CoSet<Tags>>,
}

#[co]
pub enum PinAction {
	Pin(Cid, Tags),
	Unpin(Cid, Tags),
	UnpinAll(Tags),
}

impl Reducer<PinAction> for Pin {
	async fn reduce(
		state: OptionLink<Self>,
		event: Link<ReducerAction<PinAction>>,
		storage: &CoreBlockStorage,
	) -> Result<Link<Self>, anyhow::Error> {
		let action = storage.get_value(&event).await?;
		let mut result = storage.get_value_or_default(&state).await?;
		match &action.payload {
			PinAction::Pin(cid, tags) => {
				reduce_pin(&mut result.pins, storage, cid, tags).await?;
			},
			PinAction::Unpin(cid, tags) => {
				unpin(&mut result.pins, storage, cid, tags).await?;
			},
			PinAction::UnpinAll(tags) => {
				reduce_unpin_all(&mut result.pins, storage, tags).await?;
			},
		}
		Ok(storage.set_value(&result).await?)
	}
}

async fn reduce_pin(
	pins: &mut CoMap<Cid, CoSet<Tags>>,
	storage: &CoreBlockStorage,
	cid: &Cid,
	tags: &Tags,
) -> Result<(), anyhow::Error> {
	// get current or new
	let mut pinned_tags = pins.get(storage, cid).await?.unwrap_or_default();

	// insert new tag into the set
	pinned_tags.insert(storage, tags.clone()).await?;

	// update map
	pins.insert(storage, *cid, pinned_tags).await?;
	Ok(())
}

async fn unpin(
	pins: &mut CoMap<Cid, CoSet<Tags>>,
	storage: &CoreBlockStorage,
	cid: &Cid,
	tags: &Tags,
) -> Result<(), anyhow::Error> {
	// get current tags for cid
	if let Some(mut pinned_tags) = pins.get(storage, cid).await? {
		// remove given tag from set
		pinned_tags.remove(storage, tags.clone()).await?;

		// check if set is now empty
		if pinned_tags.is_empty() {
			// last tags removed from set -> remove cid from map
			pins.remove(storage, *cid).await?;
		} else {
			// update map with filtered tags
			pins.insert(storage, *cid, pinned_tags).await?;
		}
	}
	Ok(())
}

async fn reduce_unpin_all(
	pins: &mut CoMap<Cid, CoSet<Tags>>,
	storage: &CoreBlockStorage,
	tags: &Tags,
) -> Result<(), anyhow::Error> {
	// collect all cids that have matching tags
	let cids_to_unpin: Vec<Cid> = pins
		.stream(storage)
		.try_filter_map(|(cid, pin_tag_set): (Cid, CoSet<Tags>)| {
			let tags = tags.clone();
			let storage = storage.clone();
			async move {
				// check if any tag in the set matches
				let has_match = pin_tag_set
					.stream(&storage)
					.try_any(|pin_tags| std::future::ready(tags.matches(&pin_tags)))
					.await
					.unwrap_or(false);
				Ok(if has_match { Some(cid) } else { None })
			}
		})
		.try_collect()
		.await?;

	// unpin matched cids
	for cid in &cids_to_unpin {
		unpin(pins, storage, cid, tags).await?;
	}
	Ok(())
}
