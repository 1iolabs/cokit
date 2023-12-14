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
	use crate::{library::entry::EntryBlock, Clock, Entry};
	use co_storage::BlockSerializer;
	use serde::Serialize;
	use std::cmp::Ordering;

	#[derive(Debug, Serialize)]
	struct Test {
		v: i32,
	}
	fn create_test_entry(v: i32, identity: char, time: u64) -> EntryBlock {
		let payload = BlockSerializer::default().serialize(&Test { v }).unwrap();
		let entry = Entry {
			id: "test".to_string().into_bytes(),
			payload: payload.cid().clone(),
			next: vec![],
			refs: vec![],
			clock: Clock::new(vec![identity as u8], time),
		};
		EntryBlock::from_entry(entry).unwrap()
	}

	#[test]
	fn test_last_write_wins_returns_less_when_a_is_less_then_b() {
		let a = create_test_entry(1, 'A', 1);
		let b = create_test_entry(1, 'B', 2);
		assert_eq!(Ordering::Less, last_write_wins(&a, &b));
	}

	#[test]
	fn test_last_write_wins_returns_greater_when_a_is_greater_then_b() {
		let a = create_test_entry(1, 'A', 2);
		let b = create_test_entry(1, 'B', 1);
		assert_eq!(Ordering::Greater, last_write_wins(&a, &b));
	}

	#[test]
	fn test_last_write_wins_returns_less_when_a_is_equal_to_b() {
		let a = create_test_entry(1, 'A', 1);
		let b = create_test_entry(1, 'B', 1);
		assert_eq!(Ordering::Less, last_write_wins(&a, &b));
	}

	#[test]
	fn test_last_write_wins_returns_equal_when_a_and_b_are_the_same() {
		let a = create_test_entry(1, 'A', 1);
		let b = create_test_entry(1, 'A', 1);
		assert_eq!(Ordering::Equal, last_write_wins(&a, &b));
	}
}
