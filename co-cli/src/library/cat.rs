use cid::Cid;
use co_primitives::{KnownMultiCodec, MultiCodec};
use co_sdk::{unixfs_cat_buffer, BlockSerializer, BlockStorage, CoStorage};
use ipld_core::ipld::Ipld;
use std::io::Write;

pub async fn cat_output(storage: CoStorage, cid: Cid, pretty: bool) -> Result<(), anyhow::Error> {
	if pretty {
		let block = storage.get(&cid).await?;
		let codec = MultiCodec::from(block.cid().codec());
		if MultiCodec::is(cid, KnownMultiCodec::CoEncryptedBlock) {
			println!("Codec: {:?} ({})", Into::<MultiCodec>::into(cid.codec()), cid.codec());
			println!("Cid: {}", block.cid());
		}
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
		let cid = match MultiCodec::from(cid.codec()) {
			MultiCodec::Known(KnownMultiCodec::CoEncryptedBlock) => storage.get(&cid).await?.into_inner().0,
			_ => cid,
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
