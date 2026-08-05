// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
	) -> Result<(), ReducerFlushError>;
}

/// Why a flush failed, which decides whether the reducer actor still publishes the child state.
///
/// There is deliberately no [`From<anyhow::Error>`] conversion: every fallible boundary has to name its
/// variant, so a future unclassified `?` fails to compile instead of silently choosing commit semantics.
#[derive(Debug)]
pub enum ReducerFlushError {
	/// Failure before the child state is committed.
	Fatal(anyhow::Error),

	/// Failure after the child state is committed.
	Committed(anyhow::Error),
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
