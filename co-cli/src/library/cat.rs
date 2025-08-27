use cid::Cid;
use co_primitives::{BlockStorageSettings, CloneWithBlockStorageSettings, KnownMultiCodec, MultiCodec};
use co_sdk::{unixfs_cat_buffer, BlockSerializer, BlockStorage, CoStorage};
use ipld_core::ipld::Ipld;
use std::io::Write;

pub async fn cat_output(storage: CoStorage, cid: Cid, pretty: bool, decrypt: bool) -> Result<(), anyhow::Error> {
	if pretty {
		let block = if decrypt && MultiCodec::is(cid, KnownMultiCodec::CoEncryptedBlock) {
			let transform_storage = storage.clone_with_settings(BlockStorageSettings::new().with_transform());
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
			let ipld: Ipld = BlockSerializer::default().deserialize(&block)?;
			println!("{:#?}", ipld);
		} else {
			hexdump::hexdump(block.data());
		}
	} else {
		// encrypted?
		let storage = match MultiCodec::from(cid.codec()) {
			MultiCodec::Known(KnownMultiCodec::CoEncryptedBlock) if decrypt => {
				storage.clone_with_settings(BlockStorageSettings::new().with_transform())
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
