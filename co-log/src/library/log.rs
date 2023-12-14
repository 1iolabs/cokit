use super::{
	conflict::last_write_wins,
	entry::EntryBlock,
	heads::find_heads,
	storage::{EntryStorage, TypedStorage},
};
use crate::{library::clock::max_clock, Clock, Entry, Identity};
use anyhow::{anyhow, Context};
use libipld::Cid;
use std::collections::{BTreeSet, HashSet};

pub struct Log {
	id: Vec<u8>,

	/// Identity.
	identity: Box<dyn Identity>,

	/// Current heads.
	heads: BTreeSet<Cid>,

	/// Storage for entries.
	entry_store: EntryStorage,

	// Index of entries.
	index: HashSet<Cid>,
}

impl Log {
	pub fn new(id: Vec<u8>, identity: Box<dyn Identity>, store: EntryStorage, heads: Vec<Cid>) -> Self {
		Log { id, identity, heads: heads.into_iter().collect(), entry_store: store, index: Default::default() }
	}

	pub fn id(&self) -> &[u8] {
		&self.id
	}

	pub fn heads(&self) -> Vec<Cid> {
		self.heads.iter().cloned().collect()
	}

	pub fn get(&self, cid: &Cid) -> Option<EntryBlock> {
		self.entry_store.get(cid).ok()
	}

	// fn contains(&self, cid: &Cid) -> bool {
	// 	self.index.contains(cid)
	// }

	/// Iterate entries starting at the head.
	pub fn iter(&self) -> LogIterator {
		let stack: Result<Vec<EntryBlock>, anyhow::Error> = self
			.heads
			.iter()
			.map(|cid| -> Result<_, anyhow::Error> { self.entry_store.get(cid).context("Get entry from storage") })
			.collect();
		LogIterator::new(&self.entry_store, stack)
	}

	/// Push item as new entry.
	pub fn push(&mut self, item: Cid) -> Result<Cid, anyhow::Error> {
		// create entry
		let entry = Entry {
			id: self.id().to_vec(),
			clock: Clock::new(
				self.identity.identity().as_bytes().to_vec(),
				max_clock(
					self.heads
						.iter()
						.map(|item| -> Result<Entry, anyhow::Error> { Ok(self.entry_store.get(item)?.into()) })
						.collect::<Result<Vec<Entry>, _>>()?
						.into_iter(),
				),
			)
			.next(),
			payload: item,
			next: self.heads(),
			refs: Vec::new(),
		};
		let entry_block = EntryBlock::from_entry(self.identity.as_ref(), entry)?;
		let entry_cid = entry_block.cid().clone();

		// set state
		self.entry_store.set(entry_block)?; // to be atomic in case of error do this first
		self.index.insert(entry_cid.clone());
		self.heads_set([entry_cid.clone()].into_iter());

		// result
		Ok(entry_cid)
	}

	pub fn join_entry(&mut self, identity: &dyn Identity, entry: EntryBlock) -> Result<bool, anyhow::Error> {
		if self.index.contains(entry.cid()) {
			return Ok(false)
		}

		// verify
		verify_entry(self, identity, &entry)?;

		// calculate
		let mut entries_to_add: BTreeSet<Cid> = BTreeSet::new();
		entries_to_add.insert(entry.cid().clone());
		let mut entries_to_get: BTreeSet<Cid> = BTreeSet::new();
		entries_to_get.extend(entry.entry().next.iter());
		entries_to_get.extend(entry.entry().refs.iter());
		let mut connected_heads: BTreeSet<Cid> = BTreeSet::new();
		while !entries_to_get.is_empty() {
			// TODO: prefetch
			// self.entry_store.fetch(entries_to_get.iter())
			while let Some(cid) = entries_to_get.pop_first() {
				let e = self.entry_store.get(&cid)?;
				verify_entry(&self, identity, &e)?;
				entries_to_add.insert(e.cid().clone());

				for next in e.entry().next.iter().chain(e.entry().refs.iter()) {
					if !self.index.contains(next) && !entries_to_add.contains(next) {
						entries_to_get.insert(next.clone());
					} else if self.heads.contains(next) {
						connected_heads.insert(next.clone());
					}
				}
			}
		}

		// resolve
		let heads = self
			.heads
			.iter()
			// skip connected entries
			.filter(|cid| !connected_heads.contains(cid))
			.chain([entry.cid()].into_iter())
			.map(|cid| self.entry_store.get(cid))
			.collect::<Result<Vec<_>, _>>()?;

		// mut: index
		for cid in entries_to_add.into_iter() {
			self.index.insert(cid);
		}

		// mut: heads
		self.heads_set(find_heads(heads.iter()).iter().map(|e| e.cid().clone()));

		// result
		Ok(true)
	}

	pub fn join(&mut self, other: &Log) -> Result<(), anyhow::Error> {
		for head in other.heads.iter() {
			self.join_entry(other.identity.as_ref(), other.get(head).ok_or(anyhow!("not found: {}", head))?)?;
		}
		Ok(())
	}

	// fn heads_insert(&mut self, cid: Cid) -> Result<(), anyhow::Error> {
	// 	let entries = self
	// 		.heads
	// 		.iter()
	// 		.chain([cid].iter())
	// 		.map(|cid| self.entry_store.get(cid))
	// 		.collect::<Result<Vec<_>, _>>()?;
	// 	let new_heads = find_heads(entries.iter());
	// 	self.heads_set(new_heads.iter().map(|e| e.cid().clone()));
	// 	Ok(())
	// }

	fn heads_set(&mut self, heads: impl Iterator<Item = Cid>) {
		self.heads.clear();
		self.heads.extend(heads);
	}
}

fn verify_entry(log: &Log, identity: &dyn Identity, entry: &EntryBlock) -> Result<(), anyhow::Error> {
	// verify log
	if &entry.entry().id != log.id() {
		return Err(anyhow::anyhow!("Invalid log"))
	}

	// verify signature
	if !entry.verify(identity)? {
		return Err(anyhow::anyhow!("Invalid entry signature"))
	}

	// ok
	Ok(())
}

pub struct LogIterator<'a> {
	storage: &'a EntryStorage,
	stack: Vec<EntryBlock>,
	error: Option<anyhow::Error>,
	traversed: HashSet<Cid>,
}
impl<'a> LogIterator<'a> {
	fn new(storage: &'a EntryStorage, stack: Result<Vec<EntryBlock>, anyhow::Error>) -> Self {
		match stack {
			Ok(s) => LogIterator { storage, stack: s, error: None, traversed: Default::default() },
			Err(e) => LogIterator { storage, stack: Default::default(), error: Some(e), traversed: Default::default() },
		}
	}

	fn sort(&mut self) {
		self.stack.sort_by(last_write_wins);
	}
}
impl<'a> Iterator for LogIterator<'a> {
	type Item = Result<EntryBlock, anyhow::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		// error?
		if let Some(e) = self.error.take() {
			// clear stack because we are done after an error
			self.stack.clear();

			// return error
			return Some(Err(e))
		}

		// sort stack
		//  TODO: do we need to consider causality or is the clock enought?
		self.sort();

		// stack
		if let Some(entry) = self.stack.pop() {
			if !self.traversed.contains(entry.cid()) {
				// flag as traversed
				self.traversed.insert(entry.cid().clone());

				// TODO: (pre) fetch refs
				// self.storage.fetch(entry.entry().next.iter());
				// self.storage.fetch(entry.entry().refs.iter());

				// read next and add to stack
				let nexts: Result<Vec<EntryBlock>, anyhow::Error> = entry
					.entry()
					.next
					.iter()
					.map(|cid| -> Result<_, anyhow::Error> { self.storage.get(cid).context("Get entry from storage") })
					.collect();
				match nexts {
					Ok(mut i) => self.stack.append(&mut i),
					Err(e) => self.error = Some(e),
				}

				// result
				return Some(Ok(entry))
			}
		}

		None
	}
}
