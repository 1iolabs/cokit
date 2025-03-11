use crate::{Block, KnownMultiCodec, MultiCodec, StoreParams};
use cid::Cid;
use ipld_core::codec::Links;

#[derive(Debug, Default, Clone)]
pub struct BlockLinks {}
impl BlockLinks {
	pub fn new() -> Self {
		Self {}
	}

	/// Test if the CID codec possibly contains links.
	pub fn has_links(&self, cid: impl Into<MultiCodec>) -> bool {
		match cid.into() {
			MultiCodec::Known(KnownMultiCodec::DagPb)
			| MultiCodec::Known(KnownMultiCodec::DagCbor)
			| MultiCodec::Known(KnownMultiCodec::DagJson) => true,
			_ => false,
		}
	}

	/// Get block references.
	///
	/// # Notes
	/// - This because of the block size limit should usually small.
	/// - The same [`Cid`] possibly is referenced multiple times.
	pub fn links<'a, P: StoreParams>(
		&self,
		block: &'a Block<P>,
	) -> Result<impl Iterator<Item = Cid> + Send + Sync + use<'a, P>, anyhow::Error> {
		let iter: Box<dyn Iterator<Item = Cid> + Send + Sync> = match MultiCodec::from(block.cid()) {
			MultiCodec::Known(KnownMultiCodec::DagPb) => Box::new(ipld_dagpb::DagPbCodec::links(block.data())?),
			MultiCodec::Known(KnownMultiCodec::DagCbor) => {
				Box::new(serde_ipld_dagcbor::codec::DagCborCodec::links(block.data())?)
			},
			MultiCodec::Known(KnownMultiCodec::DagJson) => {
				Box::new(serde_ipld_dagjson::codec::DagJsonCodec::links(block.data())?)
			},
			_ => Box::new(std::iter::empty()),
		};
		Ok(iter)
	}
}
