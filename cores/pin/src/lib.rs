use co_api::{reduce, DagCollection, DagMap, DagSet, Reducer, Storage, Tags};
use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/**
 * COre that handles pinning and unpinning
 */
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pin {
	/**
	 * A DAG map containing all pinned content ids. Map is keyed by the referenced content ids. The value is a set
	 * of tags making pinning and unpinning idempotent. A service may use tags with unique identifiers to ensure
	 * that a cid stays pinned as it then cannot be unpinned by other services. To unpin, all tags must match.
	 */
	pub pins: DagMap<Cid, DagSet<Tags>>,
}

impl Pin {
	/**
	 * A simple function to check if a specific content id is pinned or not
	 */
	pub fn is_pinned(&self, cid: &Cid, s: &dyn Storage) -> bool {
		let pin_map = self.pins.get(s);
		if let Some(set) = pin_map.get(cid) {
			// returns true if the tag set for the cid is not empty
			return !set.get(s).is_empty();
		}
		false
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PinAction {
	Pin(Cid, Tags),
	Unpin(Cid, Tags),
}

impl Reducer for Pin {
	type Action = PinAction;
	fn reduce(self, event: &co_api::ReducerAction<Self::Action>, context: &mut dyn co_api::Context) -> Self {
		let mut result = self;
		let mut pin_map = result.pins.get(context.storage_mut());
		match &event.payload {
			PinAction::Pin(cid, tags) => {
				// get current or new
				let mut pinned_tags = pin_map.get(cid).cloned().unwrap_or_default();

				// push new tag to tags vec
				if pinned_tags.insert(context.storage_mut(), tags.clone()) {
					// update map
					pin_map.insert(*cid, pinned_tags);
				}
			},
			PinAction::Unpin(cid, tags) =>
			// get current tags for cid
				if let Some(mut pinned_tags) = pin_map.get(cid).cloned() {
					// remove given tag from array
					let filtered_tags: BTreeSet<Tags> =
						pinned_tags.iter(context.storage()).filter(|t| *t != *tags).collect();
					if filtered_tags.is_empty() {
						// last tags removed from set -> remove cid from map
						pin_map.remove(cid);
					} else {
						// update map with filtered tags
						pinned_tags.set(context.storage_mut(), filtered_tags);
						pin_map.insert(*cid, pinned_tags);
					}
				},
		};
		result.pins.set(context.storage_mut(), pin_map);
		result
	}
}

#[no_mangle]
pub extern "C" fn state() {
	reduce::<Pin>()
}
