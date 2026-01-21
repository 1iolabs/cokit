use crate::{from_cbor, Block, CoReference, KnownMultiCodec, MultiCodec};
use cid::Cid;
use ipld_core::codec::Links;
use serde::de::IgnoredAny;
use std::{collections::BTreeSet, fmt::Debug};

#[derive(Debug, Default, Clone)]
pub struct BlockLinks {
	filters: JoinFilter,
}
impl BlockLinks {
	pub fn new() -> Self {
		Self::default()
	}

	/// Filter.
	pub fn with_filter(mut self, filter: impl BlockLinksFilter + 'static) -> Self {
		self.filters = self.filters.with_filter(filter);
		self
	}

	/// Test if the CID codec possibly contains links.
	pub fn has_links(&self, cid: impl Into<MultiCodec>) -> bool {
		matches!(
			cid.into(),
			MultiCodec::Known(KnownMultiCodec::DagPb)
				| MultiCodec::Known(KnownMultiCodec::DagCbor)
				| MultiCodec::Known(KnownMultiCodec::DagJson)
				| MultiCodec::Known(KnownMultiCodec::CoReference)
		)
	}

	/// Get block references.
	///
	/// # Notes
	/// - This because of the block size limit should usually small.
	/// - The same [`Cid`] possibly is referenced multiple times.
	pub fn links<'a>(
		&self,
		block: &'a Block,
	) -> Result<impl Iterator<Item = Cid> + Send + Sync + use<'_, 'a>, anyhow::Error> {
		let iter: Box<dyn Iterator<Item = Cid> + Send + Sync> =
			if !self.filters.filter_block(block.cid(), block.data())? {
				Box::new(std::iter::empty())
			} else {
				match MultiCodec::from(block.cid()) {
					MultiCodec::Known(KnownMultiCodec::DagPb) => Box::new(ipld_dagpb::DagPbCodec::links(block.data())?),
					MultiCodec::Known(KnownMultiCodec::DagCbor) | MultiCodec::Known(KnownMultiCodec::CoReference) => {
						Box::new(serde_ipld_dagcbor::codec::DagCborCodec::links(block.data())?)
					},
					MultiCodec::Known(KnownMultiCodec::DagJson) => {
						Box::new(serde_ipld_dagjson::codec::DagJsonCodec::links(block.data())?)
					},
					_ => Box::new(std::iter::empty()),
				}
			};
		Ok(iter.filter(|cid| self.filters.filter(cid)))
	}
}

pub trait BlockLinksFilter: Debug + BlockLinksFilterClone + Send + Sync {
	/// Filter `cid`. Only cids which returned true will be returned.
	fn filter(&self, cid: &Cid) -> bool;

	/// Filter the block if its links should be resolve at all.
	fn filter_block(&self, cid: &Cid, data: &[u8]) -> Result<bool, anyhow::Error>;
}

pub trait BlockLinksFilterClone {
	fn box_clone(&self) -> Box<dyn BlockLinksFilter>;
}
impl Clone for Box<dyn BlockLinksFilter> {
	fn clone(&self) -> Self {
		self.box_clone()
	}
}
impl<T> BlockLinksFilterClone for T
where
	T: BlockLinksFilter + Clone + 'static,
{
	fn box_clone(&self) -> Box<dyn BlockLinksFilter> {
		Box::new(self.clone())
	}
}

#[derive(Debug, Default, Clone)]
pub struct JoinFilter {
	filters: Vec<Box<dyn BlockLinksFilter>>,
}
impl JoinFilter {
	/// Filter.
	pub fn with_filter(mut self, filter: impl BlockLinksFilter + 'static) -> Self {
		self.filters.push(Box::new(filter));
		self
	}
}
impl BlockLinksFilter for JoinFilter {
	fn filter(&self, cid: &Cid) -> bool {
		for filter in self.filters.iter() {
			if !filter.filter(cid) {
				return false;
			}
		}
		true
	}

	fn filter_block(&self, cid: &Cid, data: &[u8]) -> Result<bool, anyhow::Error> {
		for filter in self.filters.iter() {
			if !filter.filter_block(cid, data)? {
				return Ok(false);
			}
		}
		Ok(true)
	}
}

/// Filter out Cid links.
#[derive(Debug, Clone)]
pub struct IgnoreFilter {
	/// Ignore the specified [`Cid`]'s when found in links.
	ignore: BTreeSet<Cid>,
}
impl IgnoreFilter {
	pub fn new(ignore: BTreeSet<Cid>) -> Self {
		Self { ignore }
	}
}
impl BlockLinksFilter for IgnoreFilter {
	fn filter(&self, cid: &Cid) -> bool {
		!self.ignore.contains(cid)
	}

	fn filter_block(&self, _cid: &Cid, _data: &[u8]) -> Result<bool, anyhow::Error> {
		Ok(true)
	}
}

/// Filter out [`CoReference::Weak`] blocks.
#[derive(Debug, Default, Clone)]
pub struct WeakCoReferenceFilter {}
impl WeakCoReferenceFilter {
	pub fn new() -> Self {
		Self::default()
	}
}
impl BlockLinksFilter for WeakCoReferenceFilter {
	fn filter(&self, _cid: &Cid) -> bool {
		true
	}

	fn filter_block(&self, cid: &Cid, data: &[u8]) -> Result<bool, anyhow::Error> {
		Ok(if MultiCodec::is(cid, KnownMultiCodec::CoReference) {
			let reference: CoReference<IgnoredAny> = from_cbor(data)?;
			match reference {
				CoReference::Weak(_) => false,
			}
		} else {
			true
		})
	}
}
