use anyhow::{anyhow, Context};
use async_trait::async_trait;
use cid::Cid;
use co_actor::{Actor, ActorError, ActorHandle, Response, ResponseBackPressureStream, ResponseStream, TaskSpawner};
use co_primitives::{
	Block, BlockStat, BlockStorage, BlockStorageSettings, CloneWithBlockStorageSettings, StorageError, StoreParams,
	Tags,
};
use futures::{pin_mut, Stream, StreamExt, TryStreamExt};
use std::{collections::HashMap, mem::swap};

/// Overlay storage which buffers changes into memory or tmp storage if `blocks_max_memory` is hit.
#[derive(Debug, Clone)]
pub struct OverlayBlockStorage<S, T>
where
	S: BlockStorage + 'static,
{
	handle: ActorHandle<OverlayBlockMessage<S::StoreParams>>,

	// remember for clone_with_settings
	actor: OverlayBlocksActor<S, T>,
	blocks_max_memory: usize,
}
impl<S, T> OverlayBlockStorage<S, T>
where
	S: BlockStorage + Clone + 'static,
	T: BlockStorage<StoreParams = S::StoreParams> + Clone + 'static,
{
	/// Create overlay storage.
	pub fn new(spawner: TaskSpawner, next: S, tmp: T, blocks_max_memory: usize, skip_already_existing: bool) -> Self {
		let actor = OverlayBlocksActor { blocks_tmp: tmp, next, spawner, skip_already_existing };
		let instance = Actor::spawn_with(actor.spawner.clone(), Default::default(), actor.clone(), blocks_max_memory)
			.expect("OverlayBlocksActor to spwan");
		Self { handle: instance.handle(), actor, blocks_max_memory }
	}

	/// Consume and flush all changes to `to`.
	pub async fn flush(&self, to: S) -> Result<(), StorageError> {
		let changes = self.changes();
		pin_mut!(changes);
		while let Some(change) = changes.try_next().await? {
			match change {
				OverlayChange::Set(cid, items) => {
					to.set(Block::new_unchecked(cid, items)).await?;
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
			.stream_backpressure(10, OverlayBlockMessage::ConsumeChanges)
			.map(|result| match result {
				Ok(result) => result,
				Err(err) => Err(StorageError::Internal(err.into())),
			})
	}

	/// Consume all changes to base storage and return the changed references.
	pub fn flush_changes(&self) -> impl Stream<Item = Result<OverlayChangeReference, StorageError>> {
		self.handle
			.stream(OverlayBlockMessage::FlushChanges)
			.map(|result| match result {
				Ok(result) => result,
				Err(err) => Err(StorageError::Internal(err.into())),
			})
	}
}
#[async_trait]
impl<S, T> BlockStorage for OverlayBlockStorage<S, T>
where
	S: BlockStorage + 'static,
	T: BlockStorage<StoreParams = S::StoreParams> + 'static,
{
	type StoreParams = S::StoreParams;

	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		Ok(self
			.handle
			.request(|response| OverlayBlockMessage::Get(*cid, response))
			.await
			.map_err(|err| StorageError::Internal(err.into()))??)
	}

	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
		Ok(self
			.handle
			.request(|response| OverlayBlockMessage::Set(block, response))
			.await
			.map_err(|err| StorageError::Internal(err.into()))??)
	}

	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		Ok(self
			.handle
			.request(|response| OverlayBlockMessage::Remove(*cid, response))
			.await
			.map_err(|err| StorageError::Internal(err.into()))??)
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		Ok(self
			.handle
			.request(|response| OverlayBlockMessage::Stat(*cid, response))
			.await
			.map_err(|err| StorageError::Internal(err.into()))??)
	}
}
impl<S, T> CloneWithBlockStorageSettings for OverlayBlockStorage<S, T>
where
	S: BlockStorage + CloneWithBlockStorageSettings + 'static,
	T: BlockStorage<StoreParams = S::StoreParams> + Clone + 'static,
{
	fn clone_with_settings(&self, settings: BlockStorageSettings) -> Self {
		let actor = OverlayBlocksActor {
			blocks_tmp: self.actor.blocks_tmp.clone(),
			next: self.actor.next.clone_with_settings(settings),
			spawner: self.actor.spawner.clone(),
			skip_already_existing: self.actor.skip_already_existing,
		};
		let instance =
			Actor::spawn_with(self.actor.spawner.clone(), Default::default(), actor.clone(), self.blocks_max_memory)
				.expect("OverlayBlocksActor to spawn");
		Self { handle: instance.handle(), blocks_max_memory: self.blocks_max_memory, actor }
	}
}

#[derive(Debug, Default)]
pub struct OverlayBlocks {
	/// Pending blocks.
	blocks: HashMap<Cid, OverlayBlock>,

	/// Current block memory used for `blocks`.
	blocks_memory: usize,

	/// Max block memory allowed to use for `blocks` before flushing to `blocks_tmp`.
	blocks_max_memory: usize,
}

#[derive(Debug, Clone)]
struct OverlayBlocksActor<S, T> {
	/// Base storage.
	next: S,

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
	S: BlockStorage + Clone + 'static,
	T: BlockStorage<StoreParams = S::StoreParams> + Clone + 'static,
{
	type State = OverlayBlocks;
	type Message = OverlayBlockMessage<S::StoreParams>;
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
			OverlayBlockMessage::Get(cid, response) => match state.blocks.get(&cid) {
				Some(OverlayBlock::Memory(data)) => {
					response.respond(Ok(Block::new_unchecked(cid, data.clone())));
				},
				Some(OverlayBlock::Tmp) => {
					response.spawn_with(self.spawner.clone(), {
						let storage = self.blocks_tmp.clone();
						move || async move { Ok(storage.get(&cid).await?) }
					});
				},
				Some(OverlayBlock::Remove) => {
					response.send(Err(StorageError::NotFound(cid, anyhow!("removed")))).ok();
				},
				None => {
					response.spawn_with(self.spawner.clone(), {
						let storage = self.next.clone();
						move || async move { Ok(storage.get(&cid).await?) }
					});
				},
			},
			OverlayBlockMessage::Set(block, response) => {
				response
					.respond_execute(|| async {
						let (cid, data) = block.into_inner();

						// already existing?
						match state.blocks.get(&cid) {
							Some(OverlayBlock::Memory(_)) | Some(OverlayBlock::Tmp) => {
								return Ok(cid);
							},
							_ => {},
						}
						if self.skip_already_existing {
							match self.next.stat(&cid).await {
								Ok(_) => {
									return Ok(cid);
								},
								_ => {},
							}
						}

						// insert
						state.blocks_memory += data.len();
						state.blocks.insert(cid, OverlayBlock::Memory(data));

						// flush to tmp?
						if state.blocks_memory > state.blocks_max_memory {
							for (cid, overlay_block) in state.blocks.iter_mut() {
								// try to move to tmp
								match overlay_block {
									block @ OverlayBlock::Memory(_) => {
										// mark as tmp
										let mut tmp_block = OverlayBlock::Tmp;
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
									OverlayBlock::Tmp => {},
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
			OverlayBlockMessage::Remove(cid, response) => {
				match state.blocks.entry(cid).or_insert(OverlayBlock::Remove) {
					block @ OverlayBlock::Memory(_) => {
						// remove from memory usage
						state.blocks_memory -= block.memory_len();

						// mark as removed
						*block = OverlayBlock::Remove;

						// response
						response.respond(Ok(()));
					},
					block @ OverlayBlock::Tmp => {
						// mark as removed
						*block = OverlayBlock::Remove;

						// cleanup from tmp storage
						response.spawn_with(self.spawner.clone(), {
							let storage = self.blocks_tmp.clone();
							move || async move { Ok(storage.remove(&cid).await?) }
						});
					},
					OverlayBlock::Remove => {
						// noop: already removed
						response.respond(Ok(()));
					},
				}
			},
			OverlayBlockMessage::Stat(cid, response) => match state.blocks.get(&cid) {
				Some(OverlayBlock::Memory(data)) => {
					response.respond(Ok(BlockStat { size: data.len() as u64 }));
				},
				Some(OverlayBlock::Tmp) => {
					response.spawn_with(self.spawner.clone(), {
						let storage = self.blocks_tmp.clone();
						move || async move { Ok(storage.stat(&cid).await?) }
					});
				},
				Some(OverlayBlock::Remove) => {
					response.send(Err(StorageError::NotFound(cid, anyhow!("removed")))).ok();
				},
				None => {
					response.spawn_with(self.spawner.clone(), {
						let storage = self.next.clone();
						move || async move { Ok(storage.stat(&cid).await?) }
					});
				},
			},
			OverlayBlockMessage::ConsumeChanges(mut response) => {
				// take
				let mut blocks = HashMap::new();
				swap(&mut blocks, &mut state.blocks);
				state.blocks_memory = 0;

				// stream
				let blocks_tmp = self.blocks_tmp.clone();
				let next = self.next.clone();
				self.spawner.spawn(async move {
					for (cid, overlay_block) in blocks.into_iter() {
						if !match overlay_block {
							OverlayBlock::Memory(data) => {
								response.send(Ok(OverlayChange::Set(cid, data))).await.is_ok()
							},
							OverlayBlock::Tmp => {
								// get block from tmp
								let result = blocks_tmp.get(&cid).await.map(|block| {
									let (cid, data) = block.into_inner();
									OverlayChange::Set(cid, data)
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
			OverlayBlockMessage::FlushChanges(mut response) => {
				// TODO: move to background?
				for (cid, overlay_block) in state.blocks.drain() {
					if !match overlay_block {
						OverlayBlock::Memory(data) => {
							state.blocks_memory -= data.len();
							let block = Block::new_unchecked(cid, data);
							match self.next.set(block).await {
								Ok(cid) => response.send(Ok(OverlayChangeReference::Set(cid))).is_ok(),
								Err(err) => response.send(Err(err)).is_ok(),
							}
						},
						OverlayBlock::Tmp => {
							// get block from tmp
							let block = self.blocks_tmp.get(&cid).await;
							match block {
								// remove from tmp as it has been consumed now
								Ok(block) => {
									// set
									match self.next.set(block).await {
										Ok(cid) => {
											// clear
											self.blocks_tmp.remove(&cid).await.ok();

											// result
											response.send(Ok(OverlayChangeReference::Set(cid))).is_ok()
										},
										Err(err) => response.send(Err(err)).is_ok(),
									}
								},
								// when we not find the item in tmp verify if it already has been flushed to next
								Err(StorageError::NotFound(_, _)) => {
									match self.next.stat(&cid).await {
										Ok(_) => {
											// skip item if it already has been flushed to next
											true
										},
										Err(err) => {
											// forward tmp error
											response.send(Err(err)).is_ok()
										},
									}
								},
								Err(err) => response.send(Err(err)).is_ok(),
							}
						},
						OverlayBlock::Remove => match self.next.remove(&cid).await {
							Ok(_) => response.send(Ok(OverlayChangeReference::Remove(cid))).is_ok(),
							Err(err) => response.send(Err(err)).is_ok(),
						},
					} {
						break;
					}
				}
				response.complete().ok();
			},
		}
		Ok(())
	}
}

#[derive(Debug, Clone)]
enum OverlayBlock {
	/// In memory data.
	Memory(Vec<u8>),

	/// Stored in tmp storage.
	Tmp,

	/// Remove requested.
	Remove,
}
impl OverlayBlock {
	pub fn memory_len(&self) -> usize {
		match self {
			OverlayBlock::Memory(data) => data.len(),
			OverlayBlock::Tmp => 0,
			OverlayBlock::Remove => 0,
		}
	}

	pub fn into_memory(self) -> Option<Vec<u8>> {
		match self {
			OverlayBlock::Memory(data) => Some(data),
			OverlayBlock::Tmp => None,
			OverlayBlock::Remove => None,
		}
	}
}

#[derive(Debug)]
enum OverlayBlockMessage<P>
where
	P: StoreParams,
{
	/// Get block.
	Get(Cid, Response<Result<Block<P>, StorageError>>),

	/// Set block.
	Set(Block<P>, Response<Result<Cid, StorageError>>),

	// Remove block.
	Remove(Cid, Response<Result<(), StorageError>>),

	/// Stat Block.
	Stat(Cid, Response<Result<BlockStat, StorageError>>),

	/// Consume all changes via stream.
	ConsumeChanges(ResponseBackPressureStream<Result<OverlayChange, StorageError>>),

	/// Flush all changes to base storage and return the changes as stream.
	FlushChanges(ResponseStream<Result<OverlayChangeReference, StorageError>>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum OverlayChange {
	Set(Cid, Vec<u8>),
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
		assert!(changes.contains(&OverlayChange::Set(*block0.cid(), block0.data().to_vec())));
		assert!(changes.contains(&OverlayChange::Set(*block1.cid(), block1.data().to_vec())));
		assert!(changes.contains(&OverlayChange::Set(*block2.cid(), block2.data().to_vec())));
	}

	fn block_from_raw(data: Vec<u8>) -> Block<DefaultParams> {
		Block::<DefaultParams>::new(Cid::new_v1(KnownMultiCodec::Raw.into(), Code::Blake3_256.digest(&data)), data)
			.unwrap()
	}
}
