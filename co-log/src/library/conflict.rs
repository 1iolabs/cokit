use super::entry::EntryBlock;
use libipld::store::StoreParams;
use std::cmp::Ordering;

pub fn last_write_wins<P: StoreParams>(a: &EntryBlock<P>, b: &EntryBlock<P>) -> Ordering {
	match sort_by_clocks(a, b) {
		Ordering::Equal => match sort_by_clock_id(a, b) {
			Ordering::Equal => sort_by_cid(a, b),
			i => i,
		},
		i => i,
	}
}

pub fn sort_by_cid<P: StoreParams>(a: &EntryBlock<P>, b: &EntryBlock<P>) -> Ordering {
	a.cid().cmp(b.cid())
}

pub fn sort_by_clocks<P: StoreParams>(a: &EntryBlock<P>, b: &EntryBlock<P>) -> Ordering {
	a.entry().clock.cmp(&b.entry().clock)
}

pub fn sort_by_clock_id<P: StoreParams>(a: &EntryBlock<P>, b: &EntryBlock<P>) -> Ordering {
	a.entry().clock.id.cmp(&b.entry().clock.id)
}

#[cfg(test)]
mod tests {
	use super::last_write_wins;
	use crate::{library::entry::EntryBlock, Clock, Entry, Identity, PrivateIdentity, SignError};
	use co_primitives::BlockSerializer;
	use libipld::DefaultParams;
	use serde::Serialize;
	use std::{
		cmp::Ordering,
		collections::hash_map::DefaultHasher,
		hash::{Hash, Hasher},
	};

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
	}
	impl PrivateIdentity for TestIdentity {
		fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SignError> {
			let mut hasher = DefaultHasher::new();
			self.identity().hash(&mut hasher);
			data.hash(&mut hasher);
			Ok(hasher.finish().to_be_bytes().to_vec())
		}
	}

	#[derive(Debug, Serialize)]
	struct Test {
		v: i32,
	}
	fn create_test_entry(v: i32, identity: &'static str, time: u64) -> EntryBlock<DefaultParams> {
		let payload = BlockSerializer::default().serialize(&Test { v }).unwrap();
		let entry = Entry {
			id: "test".to_string().into_bytes(),
			payload: payload.cid().clone(),
			next: vec![],
			refs: vec![],
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
