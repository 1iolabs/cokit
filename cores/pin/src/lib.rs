use cid::Cid;
use co_api::{sync_api::Reducer, DagCollectionExt, DagMap, DagSet, DagSetExt, Tags};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PinAction {
	Pin(Cid, Tags),
	Unpin(Cid, Tags),
	UnpinAll(Tags),
}

impl Reducer for Pin {
	type Action = PinAction;
	fn reduce(self, event: &co_api::ReducerAction<Self::Action>, context: &mut dyn co_api::sync_api::Context) -> Self {
		let mut result = self;
		let mut pin_map = result.pins.collection(context.storage_mut());
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
			// unpin single cid with tags
			PinAction::Unpin(cid, tags) => {
				pin_map = unpin(pin_map, cid, tags, context);
			},
			// unpin all cid that match tags using these tags
			PinAction::UnpinAll(tags) => {
				// iterate all current pins
				for (cid, pin_tag_set) in pin_map.clone().iter() {
					// resolve tags for current cid
					let pin_tag_set = pin_tag_set.collection(context.storage());
					// check if tag set contains given tags
					for pin_tags in pin_tag_set {
						if tags.matches(&pin_tags) {
							// unpin found cid
							pin_map = unpin(pin_map, cid, tags, context);
							continue;
						}
					}
				}
			},
		};
		result.pins.set_collection(context.storage_mut(), pin_map);
		result
	}
}

fn unpin(
	mut pin_map: BTreeMap<Cid, DagSet<Tags>>,
	cid: &Cid,
	tags: &Tags,
	context: &mut dyn co_api::sync_api::Context,
) -> BTreeMap<Cid, DagSet<Tags>> {
	// get current tags for cid
	if let Some(mut pinned_tags) = pin_map.get(cid).cloned() {
		// remove given tag from array
		let filtered_tags: BTreeSet<Tags> = pinned_tags.iter(context.storage()).filter(|t| *t != *tags).collect();
		if filtered_tags.is_empty() {
			// last tags removed from set -> remove cid from map
			pin_map.remove(cid);
		} else {
			// update map with filtered tags
			pinned_tags.set_collection(context.storage_mut(), filtered_tags);
			pin_map.insert(*cid, pinned_tags);
		}
	}
	pin_map
}

#[cfg(all(feature = "core", target_arch = "wasm32", target_os = "unknown"))]
#[no_mangle]
pub extern "C" fn state() {
	co_api::sync_api::reduce::<Pin>()
}
