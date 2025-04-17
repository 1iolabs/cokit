use crate::{BlockStorageContentMapping, ExtendedBlock, ExtendedBlockOptions, ExtendedBlockStorage};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use cid::Cid;
use co_actor::{Actor, ActorError, ActorHandle, Response, ResponseBackPressureStream, ResponseStream, TaskSpawner};
use co_primitives::{
	Block, BlockStat, BlockStorage, BlockStorageSettings, CloneWithBlockStorageSettings, StorageError, Tags,
};
use futures::{pin_mut, Stream, StreamExt, TryStreamExt};
use std::{
	collections::{BTreeMap, HashMap},
	marker::PhantomData,
	mem::swap,
};

/// Overlay storage which buffers changes into memory or tmp storage if `blocks_max_memory` is hit.
#[derive(Debug, Clone)]
pub struct OverlayBlockStorage<S>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
{
	handle: ActorHandle<OverlayBlockMessage<S>>,

	/// If true every read operation that would be affected by the overlay will trigger a flush of the block to the
	/// base storage.
	flush_on_the_fly: bool,

	/// Base storage
	next: S,
}
impl<S> OverlayBlockStorage<S>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
{
	/// Create overlay storage.
	pub fn new<T>(spawner: TaskSpawner, next: S, tmp: T, blocks_max_memory: usize, skip_already_existing: bool) -> Self
	where
		T: BlockStorage<StoreParams = S::StoreParams> + Clone + 'static,
	{
		let actor = OverlayBlocksActor { _next: PhantomData, blocks_tmp: tmp, spawner, skip_already_existing };
		let instance = Actor::spawn_with(actor.spawner.clone(), Default::default(), actor, blocks_max_memory)
			.expect("OverlayBlocksActor to spwan");
		Self { handle: instance.handle(), flush_on_the_fly: false, next }
	}

	pub fn with_flush_on_the_fly(mut self, flush_on_the_fly: bool) -> Self {
		self.flush_on_the_fly = flush_on_the_fly;
		self
	}

	/// Flush [`Cid`] changes to base storage.
	/// Returns a [`OverlayChangeReference`] if there was a change.
	pub async fn flush(&self, cid: Cid) -> Result<Option<OverlayChangeReference>, StorageError> {
		Ok(self
			.handle
			.request({
				let next = self.next.clone();
				move |response| OverlayBlockMessage::Flush(next, cid, response)
			})
			.await
			.map_err(|err| StorageError::Internal(err.into()))??)
	}

	/// Consume and flush all changes to `to`.
	pub async fn flush_all(&self, to: S) -> Result<(), StorageError> {
		let changes = self.changes();
		pin_mut!(changes);
		while let Some(change) = changes.try_next().await? {
			match change {
				OverlayChange::Set(cid, data, options) => {
					to.set_extended((Block::new_unchecked(cid, data), options).into()).await?;
				},
				OverlayChange::Remove(cid) => {
					to.remove(&cid).await?;
				},
			}
		}
		Ok(())
	}

	/// Consume all changes.
	pub fn changes(&self) -> impl Stream<Item = Result<OverlayChange, StorageError>> {
		// TODO: make sure block are avilable all time...
		self.handle
			.stream_backpressure(10, {
				let next = self.next.clone();
				move |response| OverlayBlockMessage::ConsumeChanges(next, response)
			})
			.map(|result| match result {
				Ok(result) => result,
				Err(err) => Err(StorageError::Internal(err.into())),
			})
	}

	/// Consume all changes to base storage and return the changed references.
	pub fn flush_changes(&self) -> impl Stream<Item = Result<OverlayChangeReference, StorageError>> {
		self.handle
			.stream({
				let next = self.next.clone();
				move |response| OverlayBlockMessage::FlushChanges(next, response)
			})
			.map(|result| match result {
				Ok(result) => result,
				Err(err) => Err(StorageError::Internal(err.into())),
			})
	}
}
#[async_trait]
impl<S> BlockStorage for OverlayBlockStorage<S>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
{
	type StoreParams = S::StoreParams;

	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		if self.flush_on_the_fly {
			match self.flush(*cid).await? {
				Some(OverlayChangeReference::Set(_)) | None => Ok(self.next.get(cid).await?),
				Some(OverlayChangeReference::Remove(_)) => Err(StorageError::NotFound(*cid, anyhow!("removed"))),
			}
		} else {
			Ok(self
				.handle
				.request(|response| OverlayBlockMessage::Get(self.next.clone(), *cid, response))
				.await
				.map_err(|err| StorageError::Internal(err.into()))??)
		}
	}

	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(self, block), fields(cid = ?block.cid()))]
	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
		Ok(self
			.handle
			.request(|response| OverlayBlockMessage::Set(self.next.clone(), block.into(), response))
			.await
			.map_err(|err| StorageError::Internal(err.into()))??)
	}

	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		Ok(self
			.handle
			.request(|response| OverlayBlockMessage::Remove(self.next.clone(), *cid, response))
			.await
			.map_err(|err| StorageError::Internal(err.into()))??)
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		if self.flush_on_the_fly {
			match self.flush(*cid).await? {
				Some(OverlayChangeReference::Set(_)) | None => Ok(self.next.stat(cid).await?),
				Some(OverlayChangeReference::Remove(_)) => Err(StorageError::NotFound(*cid, anyhow!("removed"))),
			}
		} else {
			Ok(self
				.handle
				.request(|response| OverlayBlockMessage::Stat(self.next.clone(), *cid, response))
				.await
				.map_err(|err| StorageError::Internal(err.into()))??)
		}
	}
}
impl<S> CloneWithBlockStorageSettings for OverlayBlockStorage<S>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + CloneWithBlockStorageSettings + 'static,
{
	fn clone_with_settings(&self, settings: BlockStorageSettings) -> Self {
		Self {
			handle: self.handle.clone(),
			flush_on_the_fly: self.flush_on_the_fly,
			next: self.next.clone_with_settings(settings),
		}
	}
}
#[async_trait]
impl<S> BlockStorageContentMapping for OverlayBlockStorage<S>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
{
	async fn is_content_mapped(&self) -> bool {
		// just foward
		self.next.is_content_mapped().await
	}

	async fn to_plain(&self, mapped: &Cid) -> Option<Cid> {
		if self.flush_on_the_fly {
			match self.flush(*mapped).await.ok()? {
				Some(OverlayChangeReference::Set(_)) | None => self.next.to_plain(mapped).await,
				Some(OverlayChangeReference::Remove(_)) => {
					Err(StorageError::NotFound(*mapped, anyhow!("removed"))).ok()
				},
			}
		} else {
			self.handle
				.request(|r| OverlayBlockMessage::ToPlain(self.next.clone(), *mapped, r))
				.await
				.ok()?
				.ok()?
		}
	}

	async fn to_mapped(&self, plain: &Cid) -> Option<Cid> {
		// we can not have new blocks mapped in overlay because for the mapping they need to be in the base storage
		self.next.to_mapped(plain).await
	}

	async fn insert_mappings(&self, mappings: BTreeMap<Cid, Cid>) {
		// just foward we do not overlay mappings
		self.next.insert_mappings(mappings).await;
	}
}
#[async_trait]
impl<S> ExtendedBlockStorage for OverlayBlockStorage<S>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
{
	async fn set_extended(&self, block: ExtendedBlock<Self::StoreParams>) -> Result<Cid, StorageError> {
		Ok(self
			.handle
			.request(|response| OverlayBlockMessage::Set(self.next.clone(), block.into(), response))
			.await
			.map_err(|err| StorageError::Internal(err.into()))??)
	}

	async fn clear(&self) -> Result<(), StorageError> {
		Ok(self
			.handle
			.request(|response| OverlayBlockMessage::Clear(self.next.clone(), response))
			.await
			.map_err(|err| StorageError::Internal(err.into()))??)
	}
}

#[derive(Debug, Default)]
pub struct OverlayBlocks {
	/// Pending blocks.
	blocks: HashMap<Cid, OverlayBlock>,

	/// Current block memory (bytes) used for `blocks`.
	blocks_memory: usize,

	/// Max block memory (bytes) allowed to use for `blocks` before flushing to `blocks_tmp`.
	blocks_max_memory: usize,
}

#[derive(Debug, Clone)]
struct OverlayBlocksActor<S, T> {
	_next: PhantomData<S>,

	/// Temp. storage.
	blocks_tmp: T,

	/// Spawner.
	spawner: TaskSpawner,

	/// Skip to create blocks which already exist in next.
	skip_already_existing: bool,
}
#[async_trait]
impl<S, T> Actor for OverlayBlocksActor<S, T>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
	T: BlockStorage<StoreParams = S::StoreParams> + Clone + 'static,
{
	type State = OverlayBlocks;
	type Message = OverlayBlockMessage<S>;
	type Initialize = usize;

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &Tags,
		blocks_max_memory: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		let mut result = OverlayBlocks::default();
		result.blocks_max_memory = blocks_max_memory;
		Ok(result)
	}

	async fn handle(
		&self,
		_handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		match message {
			OverlayBlockMessage::Get(next, cid, response) => match state.blocks.get(&cid) {
				Some(OverlayBlock::Memory(data, _options)) => {
					response.respond(Ok(Block::new_unchecked(cid, data.clone())));
				},
				Some(OverlayBlock::Tmp(_)) => {
					response.spawn_with(self.spawner.clone(), {
						let blocks_tmp = self.blocks_tmp.clone();
						move || async move { Ok(blocks_tmp.get(&cid).await?) }
					});
				},
				Some(OverlayBlock::Remove) => {
					response.send(Err(StorageError::NotFound(cid, anyhow!("removed")))).ok();
				},
				None => {
					response.spawn_with(self.spawner.clone(), move || async move { Ok(next.get(&cid).await?) });
				},
			},
			OverlayBlockMessage::Set(next, extended_block, response) => {
				response
					.respond_execute(|| async {
						let block = extended_block.block;
						let (cid, data) = block.into_inner();

						// already existing?
						match state.blocks.get(&cid) {
							Some(OverlayBlock::Memory(_, _)) | Some(OverlayBlock::Tmp(_)) => {
								return Ok(cid);
							},
							_ => {},
						}
						if self.skip_already_existing {
							match next.stat(&cid).await {
								Ok(_) => {
									return Ok(cid);
								},
								_ => {},
							}
						}

						// insert
						state.blocks_memory += data.len();
						state.blocks.insert(cid, OverlayBlock::Memory(data, extended_block.options));

						// flush to tmp?
						if state.blocks_memory > state.blocks_max_memory {
							for (cid, overlay_block) in state.blocks.iter_mut() {
								// try to move to tmp
								match overlay_block {
									block @ OverlayBlock::Memory(_, _) => {
										// mark as tmp
										let mut tmp_block = OverlayBlock::Tmp(block.options().unwrap_or_default());
										swap(block, &mut tmp_block);

										// apply
										if let Some(data) = tmp_block.into_memory() {
											// decrease memory usage
											state.blocks_memory -= data.len();

											// create in tmp
											// TODO: recover block on failure?
											self.blocks_tmp
												.set(Block::new_unchecked(*cid, data))
												.await
												.with_context(|| format!("Move block to tmp failed: {:?}", cid))?;
										}
									},
									OverlayBlock::Tmp(_) => {},
									OverlayBlock::Remove => {},
								}

								// done?
								if !(state.blocks_memory > state.blocks_max_memory) {
									break;
								}
							}
						}

						// response
						Ok(cid)
					})
					.await;
			},
			OverlayBlockMessage::Remove(_next, cid, response) => {
				response
					.respond_execute(|| async {
						match state.blocks.get(&cid) {
							Some(OverlayBlock::Memory(_, _)) => {
								// clear
								let block = state.blocks.remove(&cid);
								if let Some(block) = block {
									state.blocks_memory -= block.memory_len();
								}
							},
							Some(OverlayBlock::Tmp(_)) => {
								// remove from tmp
								self.blocks_tmp.remove(&cid).await?;

								// clear
								state.blocks.remove(&cid);
							},
							Some(OverlayBlock::Remove) => {
								// noop: already removed
							},
							None => {
								// mark to remove
								state.blocks.insert(cid, OverlayBlock::Remove);
							},
						}
						Ok(())
					})
					.await;
			},
			OverlayBlockMessage::Stat(next, cid, response) => match state.blocks.get(&cid) {
				Some(OverlayBlock::Memory(data, _)) => {
					response.respond(Ok(BlockStat { size: data.len() as u64 }));
				},
				Some(OverlayBlock::Tmp(_)) => {
					response.spawn_with(self.spawner.clone(), {
						let blocks_tmp = self.blocks_tmp.clone();
						move || async move { Ok(blocks_tmp.stat(&cid).await?) }
					});
				},
				Some(OverlayBlock::Remove) => {
					response.send(Err(StorageError::NotFound(cid, anyhow!("removed")))).ok();
				},
				None => {
					response.spawn_with(self.spawner.clone(), move || async move { Ok(next.stat(&cid).await?) });
				},
			},
			OverlayBlockMessage::ToPlain(next, cid, response) => {
				let overlay_result = match state.blocks.get(&cid) {
					Some(OverlayBlock::Memory(_, _)) | Some(OverlayBlock::Tmp(_)) => {
						Err(StorageError::Internal(anyhow!("overlay: not flushed yet")))
					},
					Some(OverlayBlock::Remove) => Err(StorageError::NotFound(cid, anyhow!("overlay: removed"))),
					None => Ok(()),
				};
				response.spawn_with(self.spawner.clone(), {
					move || async move {
						overlay_result?;
						Ok(next.to_plain(&cid).await)
					}
				});
			},
			OverlayBlockMessage::Flush(next, cid, response) => {
				match state.blocks.remove(&cid) {
					Some(block) => {
						// state
						state.blocks_memory -= block.memory_len();

						// flush
						response.spawn_with(&self.spawner, {
							let blocks_tmp = self.blocks_tmp.clone();
							move || async move { Ok(Some(flush_block(&next, &blocks_tmp, cid, block).await?)) }
						});
					},
					None => {
						response.respond(Ok(None));
					},
				}
			},
			OverlayBlockMessage::ConsumeChanges(next, mut response) => {
				// take
				let mut blocks = HashMap::new();
				swap(&mut blocks, &mut state.blocks);
				state.blocks_memory = 0;

				// stream
				let blocks_tmp = self.blocks_tmp.clone();
				self.spawner.spawn(async move {
					for (cid, overlay_block) in blocks.into_iter() {
						if !match overlay_block {
							OverlayBlock::Memory(data, options) => {
								response.send(Ok(OverlayChange::Set(cid, data, options))).await.is_ok()
							},
							OverlayBlock::Tmp(options) => {
								// get block from tmp
								let result = blocks_tmp.get(&cid).await.map(|block| {
									let (cid, data) = block.into_inner();
									OverlayChange::Set(cid, data, options)
								});
								match &result {
									// remove from tmp as it has been consumed now
									Ok(_) => {
										blocks_tmp.remove(&cid).await.ok();
									},
									// when we not find the item in tmp verify if it already has been flushed to next
									Err(StorageError::NotFound(_, _)) => {
										match next.stat(&cid).await {
											Ok(_) => {
												// skip item if it already has been flushed to next
												continue;
											},
											Err(_) => {
												// forward tmp error
											},
										}
									},
									_ => (),
								}

								// send
								response.send(result).await.is_ok()
							},
							OverlayBlock::Remove => response.send(Ok(OverlayChange::Remove(cid))).await.is_ok(),
						} {
							break;
						}
					}
					response.complete().ok();
				});
			},
			OverlayBlockMessage::FlushChanges(next, mut response) => {
				// TODO: move to background?
				for (cid, overlay_block) in state.blocks.drain() {
					if !match flush_block(&next, &self.blocks_tmp, cid, overlay_block).await {
						Ok(change) => response.send(Ok(change)).is_ok(),
						Err(err) => response.send(Err(err)).is_ok(),
					} {
						break;
					}
				}
				response.complete().ok();
			},
			OverlayBlockMessage::Clear(next, response) => {
				response
					.respond_execute(|| async {
						// local
						state.blocks.clear();
						state.blocks_memory = 0;

						// tmp
						// TODO: self.blocks_tmp.clear().await?;

						// next
						next.clear().await?;
						Ok(())
					})
					.await;
			},
		}
		Ok(())
	}
}

async fn flush_block<S, T>(
	next: &S,
	blocks_tmp: &T,
	cid: Cid,
	block: OverlayBlock,
) -> Result<OverlayChangeReference, StorageError>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
	T: BlockStorage<StoreParams = S::StoreParams> + Clone + 'static,
{
	match block {
		OverlayBlock::Memory(data, options) => {
			let block = Block::new_unchecked(cid, data.clone());

			// flush
			tracing::trace!(?cid, "overlay-flush-set");
			next.set_extended((block, options).into()).await?;

			Ok(OverlayChangeReference::Set(cid))
		},
		OverlayBlock::Tmp(options) => {
			// block
			let block = blocks_tmp.get(&cid).await?;

			// flush
			tracing::trace!(?cid, "overlay-flush-set-from-tmp");
			next.set_extended((block, options).into()).await?;

			Ok(OverlayChangeReference::Set(cid))
		},
		OverlayBlock::Remove => {
			// flush
			tracing::trace!(?cid, "overlay-flush-remove");
			next.remove(&cid).await?;

			Ok(OverlayChangeReference::Remove(cid))
		},
	}
}

#[derive(Debug, Clone)]
enum OverlayBlock {
	/// In memory data.
	Memory(Vec<u8>, ExtendedBlockOptions),

	/// Stored in tmp storage.
	Tmp(ExtendedBlockOptions),

	/// Remove requested.
	Remove,
}
impl OverlayBlock {
	pub fn memory_len(&self) -> usize {
		match self {
			OverlayBlock::Memory(data, _) => data.len(),
			OverlayBlock::Tmp(_) => 0,
			OverlayBlock::Remove => 0,
		}
	}

	pub fn into_memory(self) -> Option<Vec<u8>> {
		match self {
			OverlayBlock::Memory(data, _) => Some(data),
			OverlayBlock::Tmp(_) => None,
			OverlayBlock::Remove => None,
		}
	}

	pub fn options(&self) -> Option<ExtendedBlockOptions> {
		match self {
			OverlayBlock::Memory(_, options) => Some(options.clone()),
			OverlayBlock::Tmp(options) => Some(options.clone()),
			OverlayBlock::Remove => None,
		}
	}
}

#[derive(Debug)]
enum OverlayBlockMessage<S>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
{
	/// Get block.
	Get(S, Cid, Response<Result<Block<S::StoreParams>, StorageError>>),

	/// Set block.
	Set(S, ExtendedBlock<S::StoreParams>, Response<Result<Cid, StorageError>>),

	// Remove block.
	Remove(S, Cid, Response<Result<(), StorageError>>),

	/// Stat Block.
	Stat(S, Cid, Response<Result<BlockStat, StorageError>>),

	/// [`BlockStorageContentMapping::to_plain`]
	ToPlain(S, Cid, Response<Result<Option<Cid>, StorageError>>),

	/// Flush block to next storage.
	///
	/// Returns a [`OverlayChangeReference`] if the block was existing in the overlay and has been flushed.
	Flush(S, Cid, Response<Result<Option<OverlayChangeReference>, StorageError>>),

	/// Consume all changes via stream.
	ConsumeChanges(S, ResponseBackPressureStream<Result<OverlayChange, StorageError>>),

	/// Flush all changes to base storage and return the changes as stream.
	FlushChanges(S, ResponseStream<Result<OverlayChangeReference, StorageError>>),

	/// Clear storage by removing all blocks.
	Clear(S, Response<Result<(), StorageError>>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum OverlayChange {
	Set(Cid, Vec<u8>, ExtendedBlockOptions),
	Remove(Cid),
}

#[derive(Debug, Clone, PartialEq)]
pub enum OverlayChangeReference {
	Set(Cid),
	Remove(Cid),
}

#[cfg(test)]
mod tests {
	use crate::{storage::overlay::OverlayChange, MemoryBlockStorage, OverlayBlockStorage};
	use cid::Cid;
	use co_primitives::{Block, BlockStorage, DefaultParams, KnownMultiCodec};
	use futures::TryStreamExt;
	use multihash_codetable::{Code, MultihashDigest};

	#[tokio::test]
	async fn smoke() {
		let next = MemoryBlockStorage::default();
		let tmp = MemoryBlockStorage::default();
		let storage = OverlayBlockStorage::new(Default::default(), next.clone(), tmp.clone(), 8, true);
		let block0 = block_from_raw([0, 0, 0, 1].to_vec());
		let block1 = block_from_raw([0, 0, 1, 1].to_vec());
		let block2 = block_from_raw([0, 1, 1, 1].to_vec());
		let block3 = block_from_raw([1, 1, 1, 1].to_vec());

		storage.set(block0.clone()).await.unwrap();
		assert!(next.is_empty().await);
		assert!(tmp.is_empty().await);
		assert_eq!(storage.get(block0.cid()).await.unwrap(), block0);

		storage.set(block1.clone()).await.unwrap();
		assert!(next.is_empty().await);
		assert!(tmp.is_empty().await);
		assert_eq!(storage.get(block1.cid()).await.unwrap(), block1);

		// above threshold should be moved to tmp
		storage.set(block2.clone()).await.unwrap();
		assert!(next.is_empty().await);
		assert!(!tmp.is_empty().await);
		assert_eq!(storage.get(block2.cid()).await.unwrap(), block2);

		// already set should be ignored
		next.set(block3.clone()).await.unwrap();
		storage.set(block3.clone()).await.unwrap();
		assert_eq!(storage.get(block3.cid()).await.unwrap(), block3);

		// validate
		let changes = storage.changes().try_collect::<Vec<_>>().await.unwrap();
		assert_eq!(changes.len(), 3);
		assert!(changes.contains(&OverlayChange::Set(*block0.cid(), block0.data().to_vec(), Default::default())));
		assert!(changes.contains(&OverlayChange::Set(*block1.cid(), block1.data().to_vec(), Default::default())));
		assert!(changes.contains(&OverlayChange::Set(*block2.cid(), block2.data().to_vec(), Default::default())));
	}

	fn block_from_raw(data: Vec<u8>) -> Block<DefaultParams> {
		Block::<DefaultParams>::new(Cid::new_v1(KnownMultiCodec::Raw.into(), Code::Blake3_256.digest(&data)), data)
			.unwrap()
	}
}
