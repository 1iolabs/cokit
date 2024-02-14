use co_api::{CreateLink, DagMap, DagSet, FromLink, LinkIterator, Reducer, Storage, Tags};
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
		let pin_map = self.pins.from_link(s);
		if let Some(set) = pin_map.get(cid) {
			// returns true if the tag set for the cid is not empty
			return !set.from_link(s).is_empty();
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
		let s = context.storage_mut();
		let mut pin_map = self.pins.from_link(s);
		match &event.payload {
			PinAction::Pin(cid, tags) =>
			// get tags for the given cid
				if let Some(dag_tags) = pin_map.get(cid).cloned().as_mut() {
					let mut current_tags = dag_tags.from_link(s);
					// push new tag to tags vec
					current_tags.insert(tags.clone());
					// update map
					pin_map.insert(*cid, DagSet::to_link(current_tags, s));
				} else {
					// no tags found -> create new set instead
					let mut new_set = BTreeSet::new();
					new_set.insert(tags.clone());
					// update map
					pin_map.insert(*cid, DagSet::to_link(new_set, s));
				},
			PinAction::Unpin(cid, tags) =>
			// get current tags for cid
				if let Some(current_tags) = pin_map.get(cid).cloned() {
					// remove given tag from array
					let filtered_tags: BTreeSet<Tags> =
						current_tags.iter(s).filter_map(|t| t.ok()).filter(|t| *t != *tags).collect();
					if filtered_tags.is_empty() {
						// last tags removed from set -> remove cid from map
						pin_map.remove(cid);
					} else {
						// update map with filtered tags
						let dag_tags = DagSet::to_link(filtered_tags, s);
						pin_map.insert(*cid, dag_tags);
					}
				} else {
					// Not currently pinned, cannot unpin
				},
		};
		// TODO: update instead of rewrite?
		Self { pins: DagMap::to_link(pin_map, s) }
	}
}
