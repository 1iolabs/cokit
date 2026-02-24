// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

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
pub struct CoSet {
	pub root: Option<CoCid>,
}
impl CoSet {
	#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(sync))]
	pub fn new(root: Option<CoCid>) -> Self {
		Self { root }
	}

	pub fn is_empty(&self) -> Result<bool, CoError> {
		Ok(to_set(self)?.is_empty())
	}

	pub async fn contains(&self, storage: &BlockStorage, key: Vec<u8>) -> Result<bool, CoError> {
		let key: TagValue = from_cbor(&key).map_err(CoError::new)?;
		let set = to_set(self)?
			.open(storage)
			.await
			.map_err(|err| anyhow::anyhow!("open failed: {:?}", err))?;
		let value = set
			.contains(&key)
			.await
			.map_err(|err| anyhow::anyhow!("contains_key failed: {:?}", err))?;
		Ok(value)
	}

	pub async fn insert(&self, storage: &BlockStorage, value: Vec<u8>) -> Result<Self, CoError> {
		let value: TagValue = from_cbor(&value).map_err(CoError::new)?;
		let mut set = to_set(self)?
			.open(storage)
			.await
			.map_err(|err| anyhow::anyhow!("open failed: {:?}", err))?;
		set.insert(value)
			.await
			.map_err(|err| anyhow::anyhow!("contains_key failed: {:?}", err))?;
		let map = set.store().await.map_err(|err| anyhow::anyhow!("store failed: {:?}", err))?;
		let root = Into::<Option<cid::Cid>>::into(&map).map(CoCid::from);
		Ok(CoSet { root })
	}

	pub async fn entries(
		&self,
		storage: &BlockStorage,
		skip: Option<usize>,
		limit: Option<usize>,
	) -> Result<Vec<Vec<u8>>, CoError> {
		let map = to_set(self)?;
		let mut stream = map.stream(storage).boxed();
		if let Some(skip) = skip {
			stream = stream.skip(skip).boxed();
		}
		if let Some(limit) = limit {
			stream = stream.take(limit).boxed();
		}
		stream
			.map(|item| match item {
				Ok(value) => Ok(to_cbor(&value).map_err(CoError::new)?),
				Err(err) => Err(CoError::new(err)),
			})
			.try_collect()
			.await
	}

	pub async fn stream(&self, storage: &BlockStorage, sink: crate::frb_generated::StreamSink<Option<Vec<u8>>>) {
		let map = match to_set(self) {
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
					Ok(value) => {
						let value = match to_cbor(&value) {
							Ok(value) => value,
							Err(err) => {
								sink.add_error(CoError::new(err)).ok();
								break;
							},
						};
						if sink.add(Some(value)).is_err() {
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
fn to_set(map: &CoSet) -> Result<co_primitives::CoSet<TagValue>, CoError> {
	let cid = match &map.root {
		Some(cid) => Some(cid.cid()?),
		None => None,
	};
	Ok(co_primitives::CoSet::from(cid))
}
