// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{BlockStorage, CoCid, CoError};
use co_primitives::TagValue;
use co_sdk::{from_cbor, to_cbor};
use futures::{StreamExt, TryStreamExt};

// #[derive(Debug, Clone, Hash, serde::Serialize, serde::Deserialize, PartialEq, PartialOrd, Eq, Ord)]
// pub enum TagValue {
// 	Int(i64),
// 	Bool(bool),
// 	String(String),
// }

#[derive(Debug, Default, Clone)]
pub struct CoMap {
	pub root: Option<CoCid>,
}
impl CoMap {
	#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(sync))]
	pub fn new(root: Option<CoCid>) -> Self {
		Self { root }
	}

	pub fn is_empty(&self) -> Result<bool, CoError> {
		Ok(to_map(self)?.is_empty())
	}

	#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(name = "getValue"))]
	pub async fn get(&self, storage: &BlockStorage, key: Vec<u8>) -> Result<Option<Vec<u8>>, CoError> {
		let key: TagValue = from_cbor(&key).map_err(CoError::new)?;
		let map = to_map(&self)?
			.open(storage)
			.await
			.map_err(|err| anyhow::anyhow!("open failed: {:?}", err))?;
		let value: Option<TagValue> = map.get(&key).await.map_err(|err| anyhow::anyhow!("get failed: {:?}", err))?;
		let result = match value {
			Some(value) => Some(to_cbor(&value).map_err(CoError::new)?),
			None => None,
		};
		Ok(result)
	}

	pub async fn contains(&self, storage: &BlockStorage, key: Vec<u8>) -> Result<bool, CoError> {
		let key: TagValue = from_cbor(&key).map_err(CoError::new)?;
		let map = to_map(self)?
			.open(storage)
			.await
			.map_err(|err| anyhow::anyhow!("open failed: {:?}", err))?;
		let value = map
			.contains_key(&key)
			.await
			.map_err(|err| anyhow::anyhow!("contains_key failed: {:?}", err))?;
		Ok(value)
	}

	pub async fn insert(&self, storage: &BlockStorage, key: Vec<u8>, value: Vec<u8>) -> Result<Self, CoError> {
		let key: TagValue = from_cbor(&key).map_err(CoError::new)?;
		let value: TagValue = from_cbor(&value).map_err(CoError::new)?;
		let mut map = to_map(self)?
			.open(storage)
			.await
			.map_err(|err| anyhow::anyhow!("open failed: {:?}", err))?;
		map.insert(key, value)
			.await
			.map_err(|err| anyhow::anyhow!("contains_key failed: {:?}", err))?;
		let map = map.store().await.map_err(|err| anyhow::anyhow!("store failed: {:?}", err))?;
		let root = Into::<Option<cid::Cid>>::into(&map).map(CoCid::from);
		Ok(CoMap { root })
	}

	pub async fn entries(
		&self,
		storage: &BlockStorage,
		skip: Option<usize>,
		limit: Option<usize>,
	) -> Result<Vec<(Vec<u8>, Vec<u8>)>, CoError> {
		let map = to_map(self)?;
		let mut stream = map.stream(storage).boxed();
		if let Some(skip) = skip {
			stream = stream.skip(skip).boxed();
		}
		if let Some(limit) = limit {
			stream = stream.take(limit).boxed();
		}
		stream
			.map(|item| match item {
				Ok((key, value)) => Ok((to_cbor(&key).map_err(CoError::new)?, to_cbor(&value).map_err(CoError::new)?)),
				Err(err) => Err(CoError::new(err)),
			})
			.try_collect()
			.await
	}

	pub async fn stream(
		&self,
		storage: &BlockStorage,
		sink: crate::frb_generated::StreamSink<Option<(Vec<u8>, Vec<u8>)>>,
	) {
		let map = match to_map(self) {
			Ok(map) => map,
			Err(err) => {
				sink.add_error(err).ok();
				return;
			},
		};
		let storage = storage.clone();
		let task = async move {
			let stream = map.stream(&storage);
			futures::pin_mut!(stream);
			while let Some(item) = stream.next().await {
				match item {
					Ok((key, value)) => {
						let key = match to_cbor(&key) {
							Ok(value) => value,
							Err(err) => {
								sink.add_error(CoError::new(err)).ok();
								break;
							},
						};
						let value = match to_cbor(&value) {
							Ok(value) => value,
							Err(err) => {
								sink.add_error(CoError::new(err)).ok();
								break;
							},
						};
						if sink.add(Some((key, value))).is_err() {
							return;
						}
					},
					Err(err) => {
						sink.add_error(CoError::new(err)).ok();
						break;
					},
				}
			}
			sink.add(None).ok();
		};
		flutter_rust_bridge::spawn(task);
	}
}

#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(ignore))]
fn to_map(map: &CoMap) -> Result<co_primitives::CoMap<TagValue, TagValue>, CoError> {
	let cid = match &map.root {
		Some(cid) => Some(cid.cid()?),
		None => None,
	};
	Ok(co_primitives::CoMap::from(cid))
}
