use crate::library::{
	to_external_cid::{to_external_cid_opt, to_external_cid_opt_force, to_external_cids, to_external_cids_opt_force},
	to_internal_cid::{to_internal_cid_opt, to_internal_cid_opt_force, to_internal_cids, to_internal_cids_opt_force},
};
use anyhow::anyhow;
use cid::Cid;
use co_core_co::Co;
use co_core_membership::CoState;
use co_primitives::{CoReference, OptionLink, WeakCid};
use co_storage::{
	BlockStorage, BlockStorageContentMapping, BlockStorageExt, ExtendedBlock, ExtendedBlockStorage, StorageError,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CoReducerState(pub Option<Cid>, pub BTreeSet<Cid>);
impl CoReducerState {
	pub fn new(state: Option<Cid>, heads: BTreeSet<Cid>) -> Self {
		Self(state, heads)
	}

	pub fn new_weak(state: Option<WeakCid>, heads: BTreeSet<WeakCid>) -> Self {
		Self(state.map(Into::into), heads.into_iter().map(Into::into).collect())
	}

	pub async fn from_co_state<S: BlockStorage>(storage: &S, co_state: &CoState) -> Result<Self, StorageError> {
		let (state, heads) = storage.get_value(&co_state.state).await?.into_value();
		Ok(Self::new(Some(state), heads))
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_none() && self.1.is_empty()
	}

	pub fn some(&self) -> Option<(Cid, BTreeSet<Cid>)> {
		if let Some(state) = self.0 {
			Some((state, self.1.clone()))
		} else {
			None
		}
	}

	pub fn unwrap(self) -> (Cid, BTreeSet<Cid>) {
		(self.0.unwrap(), self.1)
	}

	pub fn state(&self) -> Option<Cid> {
		self.0
	}

	pub fn heads(&self) -> BTreeSet<Cid> {
		self.1.clone()
	}

	pub fn co(&self) -> OptionLink<Co> {
		self.0.into()
	}

	pub fn iter(&self) -> impl Iterator<Item = Cid> + use<'_> {
		self.0.into_iter().chain(self.1.iter().cloned())
	}

	pub async fn to_external<S: BlockStorageContentMapping>(&self, storage: &S) -> Self {
		Self(to_external_cid_opt(storage, self.0).await, to_external_cids(storage, self.1.clone()).await)
	}

	pub async fn to_external_force<S: BlockStorageContentMapping>(&self, storage: &S) -> Result<Self, anyhow::Error> {
		Ok(Self(
			if let Some(state) = self.0 {
				Some(
					to_external_cid_opt_force(storage, Some(state))
						.await
						.ok_or_else(|| anyhow!("Failed to map state: {:?}", self.0))?,
				)
			} else {
				None
			},
			to_external_cids_opt_force(storage, self.1.clone())
				.await
				.ok_or_else(|| anyhow!("Failed to map heads: {:?}", self.1))?,
		))
	}

	pub async fn to_internal<S: BlockStorageContentMapping>(&self, storage: &S) -> Self {
		Self(to_internal_cid_opt(storage, self.0).await, to_internal_cids(storage, self.1.clone()).await)
	}

	pub async fn to_internal_force<S: BlockStorageContentMapping>(&self, storage: &S) -> Result<Self, anyhow::Error> {
		Ok(Self(
			if let Some(state) = self.0 {
				Some(
					to_internal_cid_opt_force(storage, Some(state))
						.await
						.ok_or_else(|| anyhow!("Failed to map state: {:?}", self.0))?,
				)
			} else {
				None
			},
			to_internal_cids_opt_force(storage, self.1.clone())
				.await
				.ok_or_else(|| anyhow!("Failed to map heads: {:?}", self.1))?,
		))
	}

	pub fn weak(&self) -> (Option<WeakCid>, BTreeSet<WeakCid>) {
		(self.0.map(Into::into), self.1.iter().map(WeakCid::from).collect())
	}

	/// Store reducer state into a CoState object and return it alogn with optional encryption mappings that have been
	/// applied.
	///
	/// # Args
	/// - `parent_storage` - The storage in which the CoState will be stored.
	/// - `storage` - The storage the reducer state (self) belongs to.
	pub async fn to_co_state<S: ExtendedBlockStorage + BlockStorageContentMapping, M: BlockStorageContentMapping>(
		&self,
		parent_storage: &S,
		storage: &M,
	) -> Result<Option<(CoState, Option<BTreeMap<Cid, Cid>>)>, StorageError> {
		match &self {
			CoReducerState(Some(state), heads) => {
				let block = CoReference::Weak((*state, heads.clone())).to_block()?;
				let mapping = self.to_external_mapping(storage).await;
				let link = parent_storage
					.set_extended(ExtendedBlock::new(block).with_references(mapping.clone().unwrap_or_default()))
					.await?
					.into();
				Ok(Some((CoState { state: link, encryption_mapping: None }, mapping)))
			},
			_ => Ok(None),
		}
	}

	/// Store reducer state into a CoState assuming self points to an external state.
	///
	/// # Args
	/// - `storage` - The storage in which the CoState will be stored.
	pub async fn to_external_co_state<S: BlockStorage>(&self, storage: &S) -> Result<Option<CoState>, StorageError> {
		match &self {
			CoReducerState(Some(state), heads) => {
				let block = CoReference::Weak((*state, heads.clone())).to_block()?;
				let link = storage.set(block).await?.into();
				Ok(Some(CoState { state: link, encryption_mapping: None }))
			},
			_ => Ok(None),
		}
	}

	/// Create mapping assuming self is internal.
	/// The mapping maps from internal (key) to external (value).
	pub async fn to_external_mapping<S: BlockStorageContentMapping>(&self, storage: &S) -> Option<BTreeMap<Cid, Cid>> {
		if storage.is_content_mapped().await {
			let mut map = BTreeMap::new();
			for cid in self.iter() {
				if let Some(plain) = storage.to_plain(&cid).await {
					map.insert(cid, plain);
				}
			}
			if map.is_empty() {
				None
			} else {
				Some(map)
			}
		} else {
			None
		}
	}
}
impl From<(Option<Cid>, BTreeSet<Cid>)> for CoReducerState {
	fn from(value: (Option<Cid>, BTreeSet<Cid>)) -> Self {
		Self(value.0, value.1)
	}
}
impl From<(Cid, BTreeSet<Cid>)> for CoReducerState {
	fn from(value: (Cid, BTreeSet<Cid>)) -> Self {
		Self(Some(value.0), value.1)
	}
}
impl From<CoReducerState> for (Option<Cid>, BTreeSet<Cid>) {
	fn from(value: CoReducerState) -> Self {
		(value.0, value.1)
	}
}
