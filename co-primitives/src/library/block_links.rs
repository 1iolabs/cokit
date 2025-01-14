use crate::{Block, KnownMultiCodec, MultiCodec, StoreParams};
use cid::Cid;
use ipld_core::codec::Links;

#[derive(Debug, Default, Clone)]
pub struct BlockLinks {}
impl BlockLinks {
	pub fn new() -> Self {
		Self {}
	}

	/// Test if the CID codec possible contains links.
	pub fn has_links(&self, cid: impl Into<MultiCodec>) -> bool {
		match cid.into() {
			MultiCodec::Known(KnownMultiCodec::DagPb)
			| MultiCodec::Known(KnownMultiCodec::DagCbor)
			| MultiCodec::Known(KnownMultiCodec::DagJson) => true,
			_ => false,
		}
	}

	/// Get block references.
	pub fn links<'a, P: StoreParams>(
		&self,
		block: &'a Block<P>,
	) -> Result<impl Iterator<Item = Cid> + 'a, anyhow::Error> {
		links_box(block)
	}
}

fn links_box<'a, P: StoreParams>(block: &'a Block<P>) -> Result<Box<dyn Iterator<Item = Cid> + 'a>, anyhow::Error> {
	Ok(match MultiCodec::from(block.cid()) {
		MultiCodec::Known(KnownMultiCodec::DagPb) => Box::new(ipld_dagpb::DagPbCodec::links(block.data())?),
		MultiCodec::Known(KnownMultiCodec::DagCbor) => {
			Box::new(serde_ipld_dagcbor::codec::DagCborCodec::links(block.data())?)
		},
		MultiCodec::Known(KnownMultiCodec::DagJson) => {
			Box::new(serde_ipld_dagjson::codec::DagJsonCodec::links(block.data())?)
		},
		_ => Box::new(std::iter::empty()),
	})
}
