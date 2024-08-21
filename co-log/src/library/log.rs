use super::{entry::EntryBlock, get_entry_block::get_entry_blocks, join::JoinEntry, stream::create_stream};
use crate::{library::clock::max_clock, Clock, Entry, LogError};
use co_identity::{IdentityResolverBox, PrivateIdentity};
use co_primitives::Link;
use co_storage::{BlockStorage, BlockStorageExt};
use futures::Stream;
use libipld::Cid;
use serde::Serialize;
use std::collections::{BTreeSet, HashSet};

pub struct Log<S> {
	id: Vec<u8>,

	/// Identity.
	identity_resolver: IdentityResolverBox,

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

	pub fn identity_resolver(&self) -> &IdentityResolverBox {
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
	pub fn new(id: Vec<u8>, identity_resolver: IdentityResolverBox, store: S, heads: BTreeSet<Cid>) -> Self {
		Log { id, identity_resolver, heads, entry_store: store, index: Default::default() }
	}

	/// Create new log with random ID.
	pub fn create(identity_resolver: IdentityResolverBox, store: S) -> Self {
		Self::new(uuid::Uuid::new_v4().to_bytes_le().to_vec(), identity_resolver, store, Default::default())
	}

	pub async fn get(&self, cid: &Cid) -> Result<EntryBlock<S::StoreParams>, LogError> {
		let block = self.entry_store.get(cid).await?;
		Ok(EntryBlock::from_block(block)?)
	}

	/// Iterate entries starting at the head.
	pub fn stream(&self) -> impl Stream<Item = Result<EntryBlock<S::StoreParams>, LogError>> + '_ {
		create_stream(&self.entry_store, self.heads().clone())
	}

	/// Iterate entries starting at the head.
	pub fn into_stream(self) -> impl Stream<Item = Result<EntryBlock<S::StoreParams>, LogError>> {
		async_stream::stream! {
			for await i in create_stream(&self.entry_store, self.heads().clone()) {
				yield i;
			}
		}
	}

	/// Push item as new entry.
	pub async fn push<I: PrivateIdentity + Send + Sync>(
		&mut self,
		identity: &I,
		item: Cid,
	) -> Result<Link<Entry>, LogError> {
		// heads
		let head_entries = get_entry_blocks(&self.entry_store, self.heads.iter()).await?;

		// create entry
		let entry = Entry {
			id: self.id().to_vec(),
			clock: Clock::new(
				// todo: use peerid as the identity could be used one more devices?
				identity.identity().as_bytes().to_vec(),
				max_clock(head_entries.into_iter().map(|e| e.into())),
			)
			.next(),
			payload: item,
			next: self.heads().clone(),
			refs: Default::default(),
		};
		let entry_block = EntryBlock::<S::StoreParams>::from_entry(identity, entry)?;
		let entry_cid = *entry_block.cid();

		// set state
		self.entry_store.set(entry_block.block()?).await?; // to be atomic in case of error do this first
		self.index.insert(entry_cid);
		self.heads_set([entry_cid].into_iter());

		// result
		Ok(entry_cid.into())
	}

	/// Push serializable item as new entry.
	/// Returns the `Cid` of the `Entry`.
	pub async fn push_event<T, I>(&mut self, identity: &I, item: &T) -> Result<(Link<Entry>, Link<T>), LogError>
	where
		T: Serialize + Send + Sync + Clone,
		I: PrivateIdentity + Send + Sync,
	{
		let cid = self.entry_store.set_serialized(item).await?;
		Ok((self.push(identity, cid).await?, cid.into()))
	}

	/// Join other log heads.
	///
	/// Returns true if the other heads are joined or if this call caused to load entries from storage.
	/// We can not compute if there has been changes without loading the whole log.
	pub async fn join_entry(&mut self, entry: EntryBlock<S::StoreParams>) -> Result<bool, LogError> {
		let mut join = JoinEntry::new(self.heads.clone());
		if join.join_entry(self, entry).await? {
			self.join_commit(join).await?;
			return Ok(true);
		}
		Ok(false)
	}

	/// Join other log heads.
	///
	/// Returns true if the other heads are joined or if this call caused to load entries from storage.
	/// We can not compute if there has been changes without loading the whole log.
	pub async fn join(&mut self, other: &Log<S>) -> Result<bool, LogError> {
		self.join_heads(other.heads.iter()).await
	}

	/// Join other log heads.
	///
	/// Returns true if the other heads are joined or if this call caused to load entries from storage.
	/// We can not compute if there has been changes without loading the whole log.
	pub async fn join_heads<'a>(
		&'a mut self,
		other_heads: impl Iterator<Item = &'a Cid> + 'a,
	) -> Result<bool, LogError> {
		let mut result = false;
		let entries = get_entry_blocks(&self.entry_store, other_heads).await?;
		for entry in entries {
			if self.join_entry(entry).await? {
				result = true;
			}
		}
		Ok(result)
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
