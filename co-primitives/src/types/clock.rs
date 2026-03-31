// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use serde::{Deserialize, Serialize};

/// Lamport Clock.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Clock {
	#[serde(rename = "i", with = "serde_bytes")]
	pub id: Vec<u8>,
	#[serde(rename = "t")]
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
