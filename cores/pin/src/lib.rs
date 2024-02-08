use co_api::{Content as _, DagMap, Reducer, Tags};
use libipld::Cid;
use serde::{Deserialize, Serialize};

/**
 * COre that handles pinning and unpinning
 */
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pin {
	pub pinned_events: DagMap<Cid, Vec<Tags>>,
}

impl Pin {
	pub fn is_pinned(&self, cid: &Cid) -> bool {
		let pin_map = self.pinned_events.content();
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
	fn reduce(self, event: &co_api::ReducerAction<Self::Action>, _context: &mut dyn co_api::Context) -> Self {
		let mut pin_map = self.pinned_events.content();
		match &event.payload {
			PinAction::Pin(cid, tags) =>
			// get tags for the given cid
				if let Some(current_tags) = pin_map.get(cid).cloned().as_mut() {
					// push new tag to tags vec
					current_tags.push(tags.clone());
					// update map
					pin_map.insert(*cid, current_tags.clone());
				} else {
					// no tags found -> just insert as new vector
					pin_map.insert(*cid, vec![tags.clone()]);
				},
			PinAction::Unpin(cid, tags) =>
			// get current tags for cid
				if let Some(current_tags) = pin_map.get(cid).cloned() {
					// reomve given tag from array
					let new_tags: Vec<Tags> = current_tags.into_iter().filter(|i| *i != *tags).collect();
					// TODO: validate if something got removed?

					// update map
					pin_map.insert(*cid, new_tags);
				} else {
					// Not currently pinned, cannot unpin!
					// NOTE: maybe we should return an error here?
				},
		};
		self
	}
}
