use super::{entry::EntryBlock, get_entry_block::get_entry_blocks, join::JoinEntry, stream::create_stream};
use crate::{
	library::{clock::max_clock, verify_entry::ReadOnlyEntryVerifier},
	EntryVerifier, LogError, NoEntryVerifier,
};
use cid::Cid;
use co_identity::PrivateIdentity;
use co_primitives::{BlockStorage, BlockStorageExt, Clock, Entry, Link};
use futures::{pin_mut, Stream, TryStreamExt};
use serde::Serialize;
use std::{
	collections::{BTreeSet, HashSet},
	sync::Arc,
};

#[derive(Debug, Clone)]
pub struct Log {
	id: Vec<u8>,
	entry_verifier: Arc<dyn EntryVerifier>,
	/// Current heads.
	heads: BTreeSet<Cid>,
	// Index of entries.
	index: HashSet<Cid>,
}
impl Log {
	pub fn id(&self) -> &[u8] {
		&self.id
	}

	pub fn id_string(&self) -> String {
		std::str::from_utf8(&self.id)
			.map(|s| s.to_owned())
			.unwrap_or_else(|_| format!("{:02X?}", &self.id))
	}

	pub fn heads(&self) -> &BTreeSet<Cid> {
		&self.heads
	}

	pub fn heads_iter(&self) -> impl Iterator<Item = &Cid> {
		self.heads.iter()
	}

	pub fn entry_verifier(&self) -> &dyn EntryVerifier {
		self.entry_verifier.as_ref()
	}

	/// Test if the logs currently knowns about the entry id.
	/// Note: This is not an complete view and only represents loaded/joined entries.
	pub fn contains(&self, cid: &Cid) -> bool {
		self.index.contains(cid)
	}

	/// Test if cid is currently a head.
	pub fn is_head(&self, cid: &Cid) -> bool {
		self.heads.contains(cid)
	}

	/// Clear caches.
	pub fn clear(&mut self) {
		self.index.clear();
	}
}
impl Log {
	pub fn new(id: Vec<u8>, entry_verifier: impl EntryVerifier, heads: BTreeSet<Cid>) -> Self {
		Log { id, entry_verifier: Arc::new(entry_verifier), heads, index: Default::default() }
	}

	pub fn new_local(id: Vec<u8>, heads: BTreeSet<Cid>) -> Self {
		Log { id, entry_verifier: Arc::new(NoEntryVerifier::default()), heads, index: Default::default() }
	}

	pub fn new_readonly(id: Vec<u8>, heads: BTreeSet<Cid>) -> Self {
		Log { id, entry_verifier: Arc::new(ReadOnlyEntryVerifier::default()), heads, index: Default::default() }
	}

	/// (Re)sets the heads of this log.
	pub fn set_heads(&mut self, heads: BTreeSet<Cid>) {
		self.heads = heads;
		self.index.clear();
	}

	pub async fn get<S>(&self, storage: &S, cid: &Cid) -> Result<EntryBlock, LogError>
	where
		S: BlockStorage + Clone + 'static,
	{
		let block = storage.get(cid).await?;
		Ok(EntryBlock::from_block(block)?)
	}

	/// Iterate entries starting at the head.
	pub fn stream<'a, S>(&self, storage: &'a S) -> impl Stream<Item = Result<EntryBlock, LogError>> + use<'a, S>
	where
		S: BlockStorage + Clone + 'static,
	{
		create_stream(storage, self.heads().clone())
	}

	/// Iterate entries starting at the head.
	pub fn into_stream<S>(self, storage: &S) -> impl Stream<Item = Result<EntryBlock, LogError>> + use<S>
	where
		S: BlockStorage + Clone + 'static,
	{
		let storage = storage.clone();
		async_stream::stream! {
			for await i in create_stream(&storage, self.heads().clone()) {
				yield i;
			}
		}
	}

	/// Push item as new entry.
	pub async fn push<S, I>(&mut self, storage: &S, identity: &I, item: Cid) -> Result<EntryBlock, LogError>
	where
		S: BlockStorage + Clone + 'static,
		I: PrivateIdentity + Send + Sync,
	{
		// heads
		let head_entries = get_entry_blocks(storage, self.heads.iter()).await?;

		// create entry
		let entry = Entry {
			id: self.id().to_vec(),
			clock: Clock::new(
				// todo: use peerid as the identity could be used one more devices?
				identity.identity().as_bytes().to_vec(),
				max_clock(head_entries.iter().map(|e| e.entry())),
			)
			.next(),
			payload: item,
			next: self.heads().clone(),
			refs: Default::default(),
		};
		let entry_block = EntryBlock::from_entry(identity, entry)?;
		let entry_cid = *entry_block.cid();

		// set state
		storage.set(entry_block.block()?).await?; // to be atomic in case of error do this first
		self.index.insert(entry_cid);
		self.heads_set([entry_cid].into_iter());

		// result
		Ok(entry_block)
	}

	/// Push serializable item as new entry.
	/// Returns the `Cid` of the `Entry`.
	pub async fn push_event<S, T, I>(
		&mut self,
		storage: &S,
		identity: &I,
		item: &T,
	) -> Result<(EntryBlock, Link<T>), LogError>
	where
		S: BlockStorage + Clone + 'static,
		T: Serialize + Send + Sync + Clone,
		I: PrivateIdentity + Send + Sync,
	{
		let cid = storage.set_serialized(item).await?;
		Ok((self.push(storage, identity, cid).await?, cid.into()))
	}

	/// Join other log heads.
	///
	/// Returns true if the other heads has been joined.
	pub async fn join_entry<S>(&mut self, storage: &S, entry: EntryBlock) -> Result<bool, LogError>
	where
		S: BlockStorage + Clone + 'static,
	{
		let mut join = JoinEntry::new(self.heads.clone());

		// already in log?
		if self.load(storage, &entry).await? {
			return Ok(false);
		}

		if join.join_entry(storage, self, entry).await? {
			self.join_commit(join).await?;
			return Ok(true);
		}
		Ok(false)
	}

	/// Join other log heads.
	///
	/// Returns true if the other heads has been joined.
	pub async fn join<S>(&mut self, storage: &S, other: &Log) -> Result<bool, LogError>
	where
		S: BlockStorage + Clone + 'static,
	{
		self.join_heads(storage, other.heads.iter()).await
	}

	/// Join other log heads.
	///
	/// Returns true if the other heads has been joined.
	pub async fn join_heads<'a, S>(
		&'a mut self,
		storage: &S,
		other_heads: impl IntoIterator<Item = &'a Cid> + 'a,
	) -> Result<bool, LogError>
	where
		S: BlockStorage + Clone + 'static,
	{
		let mut result = false;
		let entries = get_entry_blocks(storage, other_heads.into_iter()).await?;
		for entry in entries {
			if self.join_entry(storage, entry).await? {
				result = true;
			}
		}
		Ok(result)
	}

	async fn join_commit(&mut self, join: JoinEntry) -> Result<(), LogError> {
		let (heads, entries_to_add) = join.into_inner();

		// mut: index
		for cid in entries_to_add.into_iter() {
			self.index.insert(cid);
		}

		// mut: heads
		self.heads_set(heads.into_iter());

		//result
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
		tracing::debug!(log = ?self.id, heads = ?self.heads, "log-heads-set");
	}

	/// Make sure all entries up-to `entry` are loaded (in index).
	/// This also works with entries there are not joined yet in which case no elements are loaded.
	/// Returns `true` if the entry is found in the log (already integrated).
	async fn load<S>(&mut self, storage: &S, entry: &EntryBlock) -> Result<bool, LogError>
	where
		S: BlockStorage + Clone + 'static,
	{
		let entry_clock_time = entry.entry().clock.time;

		// already loaded?
		if self.contains(entry.cid()) {
			return Ok(true);
		}

		// use the lamport clock to detect if item is in future
		let heads = get_entry_blocks(storage, self.heads_iter()).await?;
		let max_clock_time = heads.iter().map(|head| head.entry().clock.time).max().unwrap_or(0);
		if entry_clock_time > max_clock_time {
			return Ok(false);
		}

		// go back
		let self_clone = self.clone();
		let entries = self_clone.stream(storage);
		pin_mut!(entries);
		while let Some(item) = entries.try_next().await? {
			// index
			self.index.insert(*item.cid());

			// hit item?
			if item.cid() == entry.cid() {
				return Ok(true);
			}

			// hit clock?
			if item.entry().clock.time < entry_clock_time - 1 {
				// we are before the entry clock time now so it can not be integrated already
				return Ok(false);
			}
		}

		// not found despite traversing the whole log
		Ok(false)
	}
}
