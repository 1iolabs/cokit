use crate::{
	library::{
		extract_next_heads::extract_next_heads, to_external_cid::to_external_cid, to_internal_cid::to_internal_mapped,
	},
	state::{query_core, Query, QueryExt},
	types::co_dispatch::CoDispatch,
	CoPinningKey, CoRoot, CO_CORE_NAME_STORAGE,
};
use async_trait::async_trait;
use cid::Cid;
use co_core_co::Co;
use co_core_storage::{BlockInfo, References, StorageAction};
use co_primitives::{
	tags, BlockLinks, CoDate, CoId, DynamicCoDate, IgnoreFilter, OptionLink, OptionMappedCid, Tags, WeakCid,
};
use co_storage::{BlockStorage, BlockStorageContentMapping, BlockStorageExt, ExtendedBlockStorage};
use futures::{pin_mut, TryStreamExt};
use std::{collections::BTreeSet, time::Duration};

pub const STORAGE_CO_ROOT_TYPE: &str = "co-root";
pub const STORAGE_CO_HEAD_TYPE: &str = "co-head";
pub const STORAGE_CO_STATE_TYPE: &str = "co-state";

/// Resolve shallow structure.
/// Returns all children of resolved entries.
///
/// # Args
/// - `filter` - Only include specific Cid's.
/// - `filter_pins` - Only include if BlockInfo matched any pins.
#[allow(clippy::too_many_arguments)]
#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(storage_core_storage, storage_core_dispatcher, storage, structure_resolver))]
pub async fn storage_structure<S, D>(
	storage_core_storage: &S,
	storage_core_dispatcher: &mut impl CoDispatch<StorageAction>,
	storage_core_state: OptionLink<Co>,
	storage: &D,
	date: DynamicCoDate,
	max_duration: Option<Duration>,
	filter: Option<BTreeSet<Cid>>,
	structure_resolver: &mut impl StructureResolver<S, D>,
) -> Result<(OptionLink<Co>, BTreeSet<Cid>), anyhow::Error>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
	D: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
{
	let mut result_state = storage_core_state;
	let mut result = BTreeSet::new();

	// get shallow references and blocks
	let (block_structure_pending, blocks) = query_core(CO_CORE_NAME_STORAGE)
		.with_default()
		.map(|storage_core| (storage_core.block_structure_pending, storage_core.blocks))
		.execute(storage_core_storage, storage_core_state)
		.await?;
	let block_structure_pending_stream = block_structure_pending.stream(storage_core_storage);
	pin_mut!(block_structure_pending_stream);
	let blocks = blocks.open(storage_core_storage).await?;

	// resolve
	let start = date.now_duration();
	let mut num_resolved = 0;
	let mut num_visited = 0;
	let mut num_skip_exists = 0;
	let mut num_skip_no_mapping = 0;
	let mut num_skip_failure = 0;
	let mut num_skip_filter = 0;
	let mut num_skip_structure_resolver = 0;
	while let Some((cid, pending)) = block_structure_pending_stream.try_next().await? {
		num_visited += 1;

		// filter
		if let Some(filter) = &filter {
			if !filter.contains(&cid.cid()) {
				num_skip_filter += 1;
				continue;
			}
		}

		// skip if not exists in this storage
		//  this also skips blocks that are not yet or will be never available on device
		//  because tehy are just referenced but not fetched from network.
		if !storage.exists(&cid).await? {
			num_skip_exists += 1;
			continue;
		}

		// get tags
		let block_tags = match blocks.get(&cid).await? {
			Some(block) => block.tags.clone(),
			None => Default::default(),
		};

		// map (this should return None if wrong "namespace")
		let Some(mapped_cid) = to_internal_mapped(storage, cid.into()).await else {
			num_skip_no_mapping += 1;
			continue;
		};

		// structure
		let external_links = match structure_resolver
			.resolve(storage_core_storage, pending.info(), storage, &mapped_cid, &block_tags)
			.await
		{
			Ok(StructureResolveResult::Include(external_links)) => external_links,
			Ok(StructureResolveResult::Exclude) => {
				num_skip_structure_resolver += 1;
				continue;
			},
			Err(err) => {
				tracing::warn!(?mapped_cid, ?err, "storage-structure-resolve-failed");
				num_skip_failure += 1;
				continue;
			},
		};

		// record children for net iteration
		result.extend(external_links.iter().map(|weak| weak.cid()));

		// dispatch
		//  TODO: combine actions?
		let action = StorageAction::Structure([(cid, external_links)].into());
		result_state = storage_core_dispatcher.dispatch(&action).await?.into();
		num_resolved += 1;

		// deadline?
		if let Some(max_duration) = max_duration {
			if max_duration < (date.now_duration() - start) {
				break;
			}
		}
	}

	// log
	let duration_ms: Duration = date.now_duration() - start;
	tracing::trace!(
		duration_ms = duration_ms.as_millis(),
		num_resolved,
		num_visited,
		num_children = result.len(),
		num_skip_exists,
		num_skip_no_mapping,
		num_skip_failure,
		num_skip_filter,
		num_skip_structure_resolver,
		"storage-structure"
	);

	// result
	Ok((result_state, result))
}

/// Resolve shallow structure.
/// Continue to descend into children of resolved references.
#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(storage_core_storage, storage_core_dispatcher, storage, structure_resolver))]
pub async fn storage_structure_recursive<S, D>(
	storage_core_storage: &S,
	storage_core_dispatcher: &mut impl CoDispatch<StorageAction>,
	storage_core_state: OptionLink<Co>,
	storage: &D,
	date: DynamicCoDate,
	max_duration: Option<Duration>,
	structure_resolver: &mut impl StructureResolver<S, D>,
) -> Result<(), anyhow::Error>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
	D: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
{
	let start = date.now_duration();
	let mut filter: Option<BTreeSet<Cid>> = None;

	// apply
	let mut storage_core_state = storage_core_state;
	loop {
		// apply
		let (next_storage_core_state, children) = storage_structure(
			storage_core_storage,
			storage_core_dispatcher,
			storage_core_state,
			storage,
			date.clone(),
			max_duration.map(|duration| duration - (date.now_duration() - start)),
			filter,
			structure_resolver,
		)
		.await?;
		if children.is_empty() {
			break;
		}
		filter = Some(children);
		storage_core_state = next_storage_core_state;

		// deadline?
		if let Some(max_duration) = max_duration {
			if max_duration < (date.now_duration() - start) {
				break;
			}
		}
	}

	// log
	let duration_ms: Duration = date.now_duration() - start;
	tracing::trace!(duration_ms = duration_ms.as_millis(), "storage-structure-recursive");

	// result
	Ok(())
}

pub enum StructureResolveResult {
	/// Exclude the item.
	Exclude,

	/// Include the item by using specified children references.
	/// Note: All Cids returned here are external.
	Include(References),
}

#[async_trait]
pub trait StructureResolver<S, D>: Send + Sync {
	/// Resolve links for `item`.
	///
	/// # Args
	/// - `storage_core_storage` - The storage instance which owns the storage core.
	/// - `info` - Causal block info.
	/// - `item_storage` - The storage instance which owns the item.
	/// - `item` - The Cid of the item.
	/// - `item_tags` - The tags metadata for this item.
	///
	/// # Returns
	/// A filter result.
	/// If included returns external references.
	async fn resolve(
		&mut self,
		storage_core_storage: &S,
		info: &BlockInfo,
		item_storage: &D,
		item: &OptionMappedCid,
		item_tags: &Tags,
	) -> Result<StructureResolveResult, anyhow::Error>;
}

/// Co specific structure resolver.
/// - Only follow references related to the Co.
/// - For heads do not follow [`co_log::Entry::next`] and [`co_log::Entry::refs`].
pub struct CoStructureResolver {
	root_pin: String,
	block_links: BlockLinks,
}
impl CoStructureResolver {
	pub fn new(co: &CoId, block_links: BlockLinks) -> Self {
		Self { root_pin: CoPinningKey::Root.to_string(co), block_links }
	}
}
#[async_trait]
impl<S, D> StructureResolver<S, D> for CoStructureResolver
where
	S: BlockStorage + Clone + 'static,
	D: BlockStorage + BlockStorageContentMapping + Clone + 'static,
{
	async fn resolve(
		&mut self,
		storage_core_storage: &S,
		info: &BlockInfo,
		item_storage: &D,
		item: &OptionMappedCid,
		item_tags: &Tags,
	) -> Result<StructureResolveResult, anyhow::Error> {
		let pins: BTreeSet<String> = info.pins.stream(storage_core_storage).try_collect().await?;
		if pins.contains(&self.root_pin) {
			let mut references = References::new();
			let mut block_links = self.block_links.clone();

			// sepcial types
			match item_tags.string("type") {
				Some(STORAGE_CO_ROOT_TYPE) => {
					let co_root: CoRoot = item_storage.get_deserialized(&item.internal()).await?;

					// add state with type tag
					references.extend_with_tags(
						co_root.state.iter().map(WeakCid::from),
						tags!("type": STORAGE_CO_STATE_TYPE),
					);

					// add head with type tag
					references
						.extend_with_tags(co_root.heads.iter().map(WeakCid::from), tags!("type": STORAGE_CO_HEAD_TYPE));

					// ignore state and heads (as we already added it)
					block_links =
						block_links.with_filter(IgnoreFilter::new(references.iter().map(Into::into).collect()));
				},
				Some(STORAGE_CO_HEAD_TYPE) => {
					// ignore next heads
					block_links = block_links.with_filter(IgnoreFilter::new(
						extract_next_heads(item_storage, [&item.internal()], true).await?,
					));
				},
				_ => {},
			}

			// links
			if block_links.has_links(item.internal()) {
				let block = item_storage.get(&item.internal()).await?;
				for internal_block_link in block_links.links(&block)? {
					references.insert(to_external_cid(item_storage, internal_block_link).await);
				}
			}

			// result
			Ok(StructureResolveResult::Include(references))
		} else {
			Ok(StructureResolveResult::Exclude)
		}
	}
}
