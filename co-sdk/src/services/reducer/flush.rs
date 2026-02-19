// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{reducer::core_resolver::dynamic::DynamicCoreResolver, CoReducerState, CoStorage, CoreResolver, Reducer};
use async_trait::async_trait;
use co_primitives::{Did, OptionMappedCid};
use co_storage::{BlockStorageContentMapping, ExtendedBlockStorage};
use std::collections::BTreeSet;

#[async_trait]
pub trait ReducerFlush<S, R>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + Sync + Send + 'static,
	R: CoreResolver<S> + Send + Sync + 'static,
{
	/// Flush.
	///
	/// # Args
	/// - `new_roots` - Staged new (internal) roots that have been generated.
	/// - `removed_blocks` - Staged removed blocks ([`co_storage::BlockStorage::remove`]).
	async fn flush(
		&mut self,
		storage: &S,
		reducer: &mut Reducer<S, R>,
		info: &FlushInfo,
		new_roots: Vec<CoReducerState>,
		removed_blocks: BTreeSet<OptionMappedCid>,
	) -> anyhow::Result<()>;
}

pub type CoReducerFlush = Box<dyn ReducerFlush<CoStorage, DynamicCoreResolver<CoStorage>> + Send + Sync + 'static>;

#[derive(Debug, Default, Clone)]
pub struct FlushInfo {
	/// Flushed operations that has local origin.
	pub local: bool,

	/// The last identity that executed a local operation.
	/// Only set when local it true.
	pub local_identity: Option<Did>,

	/// Whether the co has a network feature.
	pub network: bool,
}
