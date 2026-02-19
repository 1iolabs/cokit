// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use super::entry::EntryBlock;
use std::cmp::Ordering;

pub fn last_write_wins(a: &EntryBlock, b: &EntryBlock) -> Ordering {
	match sort_by_clocks(a, b) {
		Ordering::Equal => match sort_by_clock_id(a, b) {
			Ordering::Equal => sort_by_cid(a, b),
			i => i,
		},
		i => i,
	}
}

pub fn sort_by_cid(a: &EntryBlock, b: &EntryBlock) -> Ordering {
	a.cid().cmp(b.cid())
}

pub fn sort_by_clocks(a: &EntryBlock, b: &EntryBlock) -> Ordering {
	a.entry().clock.cmp(&b.entry().clock)
}

pub fn sort_by_clock_id(a: &EntryBlock, b: &EntryBlock) -> Ordering {
	a.entry().clock.id.cmp(&b.entry().clock.id)
}

#[cfg(test)]
mod tests {
	use super::last_write_wins;
	use crate::library::entry::EntryBlock;
	use co_identity::{Identity, PrivateIdentity, SignError};
	use co_primitives::{BlockSerializer, Clock, Entry};
	use serde::Serialize;
	use std::{
		cmp::Ordering,
		collections::{hash_map::DefaultHasher, BTreeSet},
		hash::{Hash, Hasher},
	};

	#[derive(Debug, Clone)]
	struct TestIdentity {
		identity: &'static str,
	}
	impl TestIdentity {
		pub fn new(identity: &'static str) -> Self {
			Self { identity }
		}
	}
	impl Identity for TestIdentity {
		fn identity(&self) -> &str {
			self.identity
		}
		fn public_key(&self) -> Option<Vec<u8>> {
			None
		}
		fn verify(&self, signature: &[u8], data: &[u8], _public_key: Option<&[u8]>) -> bool {
			signature == self.sign(data).unwrap()
		}
		fn didcomm_public(&self) -> Option<co_identity::DidCommPublicContext> {
			None
		}
		fn networks(&self) -> BTreeSet<co_primitives::Network> {
			Default::default()
		}
	}
	impl PrivateIdentity for TestIdentity {
		fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SignError> {
			let mut hasher = DefaultHasher::new();
			self.identity().hash(&mut hasher);
			data.hash(&mut hasher);
			Ok(hasher.finish().to_be_bytes().to_vec())
		}

		fn didcomm_private(&self) -> Option<co_identity::DidCommPrivateContext> {
			None
		}
	}

	#[derive(Debug, Serialize)]
	struct Test {
		v: i32,
	}
	fn create_test_entry(v: i32, identity: &'static str, time: u64) -> EntryBlock {
		let payload = BlockSerializer::default().serialize(&Test { v }).unwrap();
		let entry = Entry {
			id: "test".to_string().into_bytes(),
			payload: *payload.cid(),
			next: Default::default(),
			refs: Default::default(),
			clock: Clock::new(identity.as_bytes().to_vec(), time),
		};
		EntryBlock::from_entry(&TestIdentity::new(identity), entry).unwrap()
	}

	#[test]
	fn test_last_write_wins_returns_less_when_a_is_less_then_b() {
		let a = create_test_entry(1, "A", 1);
		let b = create_test_entry(1, "B", 2);
		assert_eq!(last_write_wins(&a, &b), Ordering::Less);
	}

	#[test]
	fn test_last_write_wins_returns_greater_when_a_is_greater_then_b() {
		let a = create_test_entry(1, "A", 2);
		let b = create_test_entry(1, "B", 1);
		assert_eq!(last_write_wins(&a, &b), Ordering::Greater);
	}

	#[test]
	fn test_last_write_wins_returns_less_when_a_is_equal_to_b() {
		let a = create_test_entry(1, "A", 1);
		let b = create_test_entry(1, "B", 1);
		assert_eq!(last_write_wins(&a, &b), Ordering::Less);
	}

	#[test]
	fn test_last_write_wins_returns_equal_when_a_and_b_are_the_same() {
		let a = create_test_entry(1, "A", 1);
		let b = create_test_entry(1, "A", 1);
		assert_eq!(last_write_wins(&a, &b), Ordering::Equal);
	}
}
