use libipld::Cid;
use serde::{Deserialize, Serialize};

mod library;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entry {
	pub id: Vec<u8>,
	pub payload: Cid,
	pub next: Vec<Cid>,
	pub refs: Vec<Cid>,
	pub clock: Clock,
}

/// Lamport Clock.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Clock {
	id: Vec<u8>,
	time: u64,
}
impl Clock {
	pub fn next(&self) -> Clock {
		Clock { id: self.id.clone(), time: self.time + 1 }
	}
}
impl PartialOrd for Clock {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		match self.time.partial_cmp(&other.time) {
			Some(core::cmp::Ordering::Equal) => {},
			ord => return ord,
		}
		self.id.partial_cmp(&other.id)
	}
}

pub struct Log {}

impl Log {
	pub fn push(data: Cid) -> Entry {}
}

trait Identity {}
