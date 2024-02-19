use co_primitives::MultiCodec;
use co_sdk::{unixfs_cat_buffer, BlockStorage, CoStorage};
use libipld::{cbor::DagCborCodec, codec::Codec, Cid, Ipld};
use std::io::Write;

pub async fn cat_output(storage: CoStorage, cid: Cid, pretty: bool) -> Result<(), anyhow::Error> {
	if pretty {
		let block = storage.get(&cid).await?;
		let codec = MultiCodec::from(block.cid().codec());
		if MultiCodec::CoEncryptedBlock == cid.codec().into() {
			println!("Codec: {:?} ({})", Into::<MultiCodec>::into(cid.codec()), cid.codec());
			println!("Cid: {}", block.cid());
		}
		println!("Codec: {:?} ({})", codec, block.cid().codec());
		println!("Size: {}", block.data().len());
		match codec {
			MultiCodec::DagCbor => {
				let ipld: Ipld = DagCborCodec::default().decode(block.data())?;
				println!("{:#?}", ipld);
			},
			_ => {
				hexdump::hexdump(block.data());
			},
		}
	} else {
		// encrypted?
		let cid = match MultiCodec::from(cid.codec()) {
			MultiCodec::CoEncryptedBlock => storage.get(&cid).await?.into_inner().0,
			_ => cid,
		};

		// print
		let mut codec = MultiCodec::from(cid.codec());
		let mut out = std::io::stdout();
		match codec {
			MultiCodec::DagPb => {
				unixfs_cat_buffer(&storage, &cid).await;
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
