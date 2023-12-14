use std::cmp::max;

use serde::{Deserialize, Serialize};

use crate::Entry;

/// Lamport Clock.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Clock {
	pub id: Vec<u8>,
	pub time: u64,
}
impl Clock {
	pub fn new(id: Vec<u8>, time: u64) -> Self {
		Self { id, time }
	}

	pub fn next(&self) -> Clock {
		Clock { id: self.id.clone(), time: self.time + 1 }
	}
}
impl PartialOrd for Clock {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}
impl Ord for Clock {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		match self.time.cmp(&other.time) {
			core::cmp::Ordering::Equal => {},
			ord => return ord,
		}
		self.id.cmp(&other.id)
	}
}

/// Finds the max clock time of the log.
/// The max clock time is equal to the tree height.
pub fn max_clock(heads: impl Iterator<Item = Entry>) -> u64 {
	heads
		.map(|head| head.clock.time)
		.reduce(|acc, head| max(acc, head))
		.unwrap_or(0)
}
