use cid::Cid;
use co_api::{async_api::Reducer, BlockStorage, BlockStorageExt, CoMap, Link, OptionLink, ReducerAction, Tags};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Storage {
	/// Block metadata.
	pub blocks: CoMap<Cid, BlockMetadata>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockMetadata {
	pub references: u32,
	pub tags: Tags,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StorageAction {
	#[serde(rename = "r")]
	Reference(Vec<Cid>),

	#[serde(rename = "u")]
	Unreference(Vec<Cid>),

	#[serde(rename = "d")]
	Remove(Vec<Cid>),

	#[serde(rename = "ti")]
	TagsInsert(Vec<Cid>, Tags),

	#[serde(rename = "tr")]
	TagsRemove(Vec<Cid>, Tags),
}

impl<S: BlockStorage + Clone + 'static> Reducer<StorageAction, S> for Storage {
	async fn reduce(
		state: OptionLink<Self>,
		event: ReducerAction<StorageAction>,
		storage: &S,
	) -> Result<Link<Self>, anyhow::Error> {
		let mut state = storage.get_value_or_default(&state).await?;
		match event.payload {
			StorageAction::Reference(cids) => {
				// // 1
				// let mut blocks = state.blocks.open_mut(storage).await?;
				// let mut block = blocks.get(&cid).await?.unwrap_or_default();
				// block.references += 1;
				// blocks.insert(cid, block).await?;
				// blocks.commit().await?;

				// // 2
				// let mut blocks = state.blocks.open(storage).await?;
				// let mut block = blocks.get(&cid).await?.unwrap_or_default();
				// block.references += 1;
				// blocks.insert(cid, block).await?;
				// state.blocks = blocks.store().await?;

				// // 3
				// let mut blocks = state.blocks.open(storage).await?;
				// blocks
				// 	.update_key(cid, |mut block| async move {
				// 		block.references += 1;
				// 		Ok(block)
				// 	})
				// 	.await?;
				// state.blocks = blocks.store().await?;

				// // 4
				// state
				// 	.blocks
				// 	.update(storage, move |mut blocks| async move {
				// 		let mut block = blocks.get(&cid).await?.unwrap_or_default();
				// 		block.references += 1;
				// 		blocks.insert(cid, block).await?;
				// 		Ok(blocks)
				// 	})
				// 	.await?;

				// // 5
				// state
				// 	.blocks
				// 	.update_key(storage, cid, |mut block| async move {
				// 		block.references += 1;
				// 		Ok(block)
				// 	})
				// 	.await?;

				let mut blocks = state.blocks.open(storage).await?;
				for cid in cids {
					blocks
						.update_key(cid, |mut block| async move {
							block.references += 1;
							Ok(block)
						})
						.await?;
				}
				state.blocks = blocks.store().await?;
			},
			StorageAction::Unreference(cids) => {
				let mut blocks = state.blocks.open(storage).await?;
				for cid in cids {
					blocks
						.update_key(cid, |mut block| async move {
							if block.references > 0 {
								block.references -= 1;
							}
							Ok(block)
						})
						.await?;
				}
				state.blocks = blocks.store().await?;
			},
			StorageAction::Remove(cids) => {
				let mut blocks = state.blocks.open(storage).await?;
				for cid in cids {
					blocks.remove(cid).await?;
				}
				state.blocks = blocks.store().await?;
			},
			StorageAction::TagsInsert(cids, tags) => {
				let mut blocks = state.blocks.open(storage).await?;
				for cid in cids {
					blocks
						.update_key(cid, |mut block| {
							let mut tags = tags.clone();
							async move {
								block.tags.append(&mut tags);
								Ok(block)
							}
						})
						.await?;
				}
				state.blocks = blocks.store().await?;
			},
			StorageAction::TagsRemove(cids, tags) => {
				let mut blocks = state.blocks.open(storage).await?;
				for cid in cids {
					blocks
						.update_key(cid, |mut block| async {
							block.tags.clear(Some(&tags));
							Ok(block)
						})
						.await?;
				}
				state.blocks = blocks.store().await?;
			},
		}
		Ok(storage.set_value(&state).await?)
	}
}
