use super::{
	entry::EntryBlock, get_entry_block::get_entry_blocks, identity::PrivateIdentity, join::JoinEntry,
	stream::create_stream,
};
use crate::{library::clock::max_clock, Clock, Entry, IdentityResolver, LogError};
use co_storage::BlockStorage;
use futures::Stream;
use libipld::Cid;
use std::collections::{BTreeSet, HashSet};

pub struct Log<S> {
	id: Vec<u8>,

	/// Identity.
	identity: Box<dyn PrivateIdentity>,
	identity_resolver: Box<dyn IdentityResolver>,

	/// Current heads.
	heads: BTreeSet<Cid>,

	/// Storage for entries.
	entry_store: S,

	// Index of entries.
	index: HashSet<Cid>,
}
impl<S> Log<S> {
	pub fn id(&self) -> &[u8] {
		&self.id
	}

	pub fn heads(&self) -> Vec<Cid> {
		self.heads.iter().cloned().collect()
	}

	pub fn identity_resolver(&self) -> &Box<dyn IdentityResolver> {
		&self.identity_resolver
	}

	pub fn storage(&self) -> &S {
		&self.entry_store
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
}
impl<S> Log<S>
where
	S: BlockStorage + Sync + Send + 'static,
{
	pub fn new(
		id: Vec<u8>,
		identity: Box<dyn PrivateIdentity>,
		identity_resolver: Box<dyn IdentityResolver>,
		store: S,
		heads: Vec<Cid>,
	) -> Self {
		Log {
			id,
			identity,
			identity_resolver,
			heads: heads.into_iter().collect(),
			entry_store: store,
			index: Default::default(),
		}
	}

	/// Create new log with random ID.
	pub fn create(identity: Box<dyn PrivateIdentity>, identity_resolver: Box<dyn IdentityResolver>, store: S) -> Self {
		Self::new(uuid::Uuid::new_v4().to_bytes_le().to_vec(), identity, identity_resolver, store, Default::default())
	}

	pub async fn get(&self, cid: &Cid) -> Result<EntryBlock<S::StoreParams>, LogError> {
		let block = self.entry_store.get(cid).await?;
		Ok(EntryBlock::from_block(block)?)
	}

	/// Iterate entries starting at the head.
	pub fn stream<'a>(&'a self) -> impl Stream<Item = Result<EntryBlock<S::StoreParams>, LogError>> + 'a {
		create_stream(&self.entry_store, self.heads())
	}

	/// Push item as new entry.
	pub async fn push(&mut self, item: Cid) -> Result<Cid, LogError> {
		// heads
		let head_entries = get_entry_blocks(&self.entry_store, self.heads.iter()).await?;

		// create entry
		let entry = Entry {
			id: self.id().to_vec(),
			clock: Clock::new(
				self.identity.identity().as_bytes().to_vec(),
				max_clock(head_entries.into_iter().map(|e| e.into())),
			)
			.next(),
			payload: item,
			next: self.heads(),
			refs: Vec::new(),
		};
		let entry_block = EntryBlock::<S::StoreParams>::from_entry(self.identity.as_ref(), entry)?;
		let entry_cid = entry_block.cid().clone();

		// set state
		self.entry_store.set(entry_block.block()?).await?; // to be atomic in case of error do this first
		self.index.insert(entry_cid.clone());
		self.heads_set([entry_cid.clone()].into_iter());

		// result
		Ok(entry_cid)
	}

	pub async fn join_entry(&mut self, entry: EntryBlock<S::StoreParams>) -> Result<bool, LogError> {
		let mut join = JoinEntry::new(self.heads.clone());
		if join.join_entry(&self, entry).await? {
			self.join_commit(join).await?;
			return Ok(true)
		}
		Ok(false)
	}

	pub async fn join(&mut self, other: &Log<S>) -> Result<(), LogError> {
		let entries = get_entry_blocks(&self.entry_store, other.heads.iter()).await?;
		for entry in entries {
			self.join_entry(entry).await?;
		}
		Ok(())
	}

	async fn join_commit(&mut self, join: JoinEntry<S>) -> Result<(), LogError> {
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
}
