// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_primitives::{
	BlockStorageCloneSettings, CloneWithBlockStorageSettings, KnownMultiCodec, MultiCodec, SignedEntry, TagValue,
	WeakCid,
};
use co_sdk::{from_cbor, to_cbor, unixfs_cat_buffer, Block, BlockStorage, CoMap, CoStorage, ReducerAction};
use futures::pin_mut;
use ipld_core::ipld::Ipld;
use serde::de::DeserializeOwned;
use std::io::Write;
use tokio_stream::StreamExt;

#[derive(Debug, Default, Clone)]
pub struct CatOptions {
	pub pretty: bool,
	pub decrypt: bool,
	pub format: Option<String>,
}
impl CatOptions {
	pub fn with_pretty(mut self, pretty: bool) -> Self {
		self.pretty = pretty;
		self
	}

	pub fn with_format(mut self, format: Option<String>) -> Self {
		self.format = format;
		self
	}

	pub fn with_decrypt(mut self, decrypt: bool) -> Self {
		self.decrypt = decrypt;
		self
	}
}

pub async fn cat_output(storage: CoStorage, cid: Cid, options: CatOptions) -> Result<(), anyhow::Error> {
	if options.pretty || options.format.is_some() {
		let block = if options.decrypt && MultiCodec::is(cid, KnownMultiCodec::CoEncryptedBlock) {
			let transform_storage = storage.clone_with_settings(BlockStorageCloneSettings::new().with_transform());
			let block = transform_storage.get(&cid).await?;
			println!("Codec: {:?} ({})", MultiCodec::from(cid), cid.codec());
			println!("Cid: {}", block.cid());
			block
		} else {
			storage.get(&cid).await?
		};
		let codec = MultiCodec::from(block.cid());
		println!("Codec: {:?} ({})", codec, block.cid().codec());
		println!("Size: {}", block.data().len());
		if MultiCodec::is_cbor(codec) {
			match print_format(&storage, &options, &block).await {
				Ok(_) => {},
				Err(err) => {
					eprintln!("---");
					eprintln!("format failed: {:?}", err);
					eprintln!("---");
					hexdump::hexdump(block.data());
				},
			}
		} else {
			hexdump::hexdump(block.data());
		}
	} else {
		// encrypted?
		let storage = match MultiCodec::from(cid.codec()) {
			MultiCodec::Known(KnownMultiCodec::CoEncryptedBlock) if options.decrypt => {
				storage.clone_with_settings(BlockStorageCloneSettings::new().with_transform())
			},
			_ => storage,
		};

		// print
		let codec = MultiCodec::from(cid.codec());
		let mut out = std::io::stdout();
		match codec {
			MultiCodec::Known(KnownMultiCodec::DagPb) => {
				out.write_all(&unixfs_cat_buffer(&storage, &cid).await?)?;
			},
			_ => {
				let block = storage.get(&cid).await?;
				out.write_all(block.data())?;
			},
		}
		out.flush()?;
	}

	// done
	Ok(())
}

async fn print_format(storage: &CoStorage, options: &CatOptions, block: &Block) -> Result<(), anyhow::Error> {
	if let Some(name) = &options.format {
		if name.starts_with("CoMap;") {
			let parts = name.split(";").collect::<Vec<_>>();
			if parts.len() != 3 {
				return Err(anyhow::anyhow!("Invalid format: `{name}` expected `CoMap;KeyType;ValueType`"));
			}
			let key_options = options.clone().with_format(Some(parts[1].to_owned()));
			let value_options = options.clone().with_format(Some(parts[2].to_owned()));
			let map = CoMap::<TagValue, TagValue>::from(Some(*block.cid()));
			let map_stream = map.stream(storage);
			pin_mut!(map_stream);
			while let Some((key, value)) = map_stream.try_next().await? {
				let key_data = to_cbor(&key)?;
				let value_data = to_cbor(&value)?;
				println!("{}={}", format_cbor(&key_options, &key_data)?, format_cbor(&value_options, &value_data)?);
			}
			return Ok(());
		} else if name.starts_with("CoList;") {
			let parts = name.split(";").collect::<Vec<_>>();
			if parts.len() != 2 {
				return Err(anyhow::anyhow!("Invalid format: `{name}` expected `CoList;ValueType`"));
			}
			let value_options = options.clone().with_format(Some(parts[1].to_owned()));
			let map = CoMap::<TagValue, TagValue>::from(Some(*block.cid()));
			let map_stream = map.stream(storage);
			pin_mut!(map_stream);
			while let Some((_key, value)) = map_stream.try_next().await? {
				let value_data = to_cbor(&value)?;
				println!("{}", format_cbor(&value_options, &value_data)?);
			}
			return Ok(());
		} else if name.starts_with("ReducerAction;") {
			let parts = name.split(";").collect::<Vec<_>>();
			if parts.len() != 2 {
				return Err(anyhow::anyhow!("Invalid format: `{name}` expected `ReducerAction;ActionType`"));
			}
			let value_options = options.clone().with_format(Some(parts[1].to_owned()));
			let value = from_cbor::<ReducerAction<TagValue>>(block.data())?;
			let payload_data = to_cbor(&value.payload)?;
			println!("Action: {:?}", value);
			println!("Payload: {}", format_cbor(&value_options, &payload_data)?);
			return Ok(());
		}
	}
	println!("{}", format_cbor(options, block.data())?);
	Ok(())
}

fn format_cbor(options: &CatOptions, data: &[u8]) -> Result<String, anyhow::Error> {
	Ok(match options.format.as_deref().unwrap_or("Ipld") {
		"co_core_co::Co" => format_cbor_debug::<co_core_co::Co>(options, data)?,
		"co_core_co::CoAction" => format_cbor_debug::<co_core_co::CoAction>(options, data)?,
		"co_core_storage::Storage" => format_cbor_debug::<co_core_storage::Storage>(options, data)?,
		"co_core_storage::StorageAction" => format_cbor_debug::<co_core_storage::StorageAction>(options, data)?,
		"co_primitives::SignedEntry" | "co-head" => format_cbor_debug::<SignedEntry>(options, data)?,
		"co_primitives::WeakCid" | "WeakCid" => format_cbor_debug::<WeakCid>(options, data)?,
		_ => format_cbor_debug::<Ipld>(options, data)?,
	})
}

fn format_cbor_debug<T: std::fmt::Debug + DeserializeOwned>(
	options: &CatOptions,
	data: &[u8],
) -> Result<String, anyhow::Error> {
	Ok(if options.pretty { format!("{:#?}", from_cbor::<T>(data)?) } else { format!("{:?}", from_cbor::<T>(data)?) })
}
