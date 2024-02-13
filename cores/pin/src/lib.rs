use co_api::{CreateLink, DagMap, DagSet, FromLink, LinkIterator, Reducer, Storage, Tags};
use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/**
 * COre that handles pinning and unpinning
 */
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pin {
	pub pins: DagMap<Cid, DagSet<Tags>>,
}

impl Pin {
	pub fn is_pinned(&self, cid: &Cid, s: &dyn Storage) -> bool {
		let pin_map = self.pins.from_link(s);
		if let Some(_) = pin_map.get(cid) {
			return true;
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
					// no tags found -> just insert as new vector
					let mut new_set = BTreeSet::new();
					new_set.insert(tags.clone());
					pin_map.insert(*cid, DagSet::to_link(new_set, s));
				},
			PinAction::Unpin(cid, tags) =>
			// get current tags for cid
				if let Some(current_tags) = pin_map.get(cid).cloned() {
					// remove given tag from array
					let resolved_tags = current_tags.iter(s).filter_map(|t| t.ok()).filter(|t| *t != *tags).collect();
					let new_tags_tags = DagSet::to_link(resolved_tags, s);
					// TODO: validate if something got removed?

					// update map
					pin_map.insert(*cid, new_tags_tags);
				} else {
					// Not currently pinned, cannot unpin!
					// NOTE: maybe we should return an error here?
				},
		};
		// TODO: update instead of rewrite
		Self { pins: DagMap::to_link(pin_map, s) }
	}
}
