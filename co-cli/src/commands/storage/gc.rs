// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{cli::Cli, library::cli_context::CliContext};
use anyhow::anyhow;
use co_core_co::Co;
use co_core_storage::BlockInfo;
use co_primitives::{CoTryStreamExt, WeakCoReferenceFilter};
use co_sdk::{
	find_co_by_pin,
	state::{query_core, Query, QueryExt},
	storage_cleanup, storage_structure_recursive, BlockStorageContentMapping, CoStructureResolver, OptionLink,
	CO_CORE_NAME_STORAGE,
};
use co_storage::ExtendedBlockStorage;
use exitcode::ExitCode;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {}

pub async fn command(context: &CliContext, cli: &Cli, _command: &Command) -> Result<ExitCode, anyhow::Error> {
	// reducer
	let application = context.application(cli).await;
	let local_co = application.local_co_reducer().await?;
	let local_storage = local_co.storage();
	let mut local_dispatcher = local_co.dispatcher(CO_CORE_NAME_STORAGE, application.local_identity());

	// resolve
	let mut last_pin = None;
	loop {
		let first_info = first_block_info_pending_blocks(&local_storage, local_co.reducer_state().await.co()).await?;
		if let Some(first_info) = first_info {
			if let Some(first_pin) = first_info.pins.stream(&local_storage).try_first().await? {
				// verify we do not loop twice
				if last_pin.as_ref() == Some(&first_pin) {
					return Err(anyhow!("Found pin after remove: {}", first_pin));
				}
				last_pin = Some(first_pin.clone());

				// resolve the co for the pin
				let co = find_co_by_pin(application.co(), first_pin).await?;
				let co_storage = co.storage();

				// output
				if !cli.quiet {
					println!("checking {} ...", co.id().as_str());
				}

				// filter
				let mut filter = CoStructureResolver::new(
					co.id(),
					application
						.co()
						.block_links(false)
						.clone()
						.with_filter(WeakCoReferenceFilter::new()),
				);

				// resolve
				storage_structure_recursive(
					&local_storage,
					&mut local_dispatcher,
					local_co.reducer_state().await.co(),
					&co_storage,
					None,
					&mut filter,
				)
				.await?;

				// remove
				if !cli.quiet {
					println!("cleaning {} ...", co.id().as_str());
				}
				storage_cleanup(
					&local_storage,
					&mut local_dispatcher,
					local_co.reducer_state().await.co(),
					&co_storage,
					&mut filter,
				)
				.await?;

				// try next
				continue;
			}
		}
		break;
	}

	// result
	Ok(exitcode::OK)
}

async fn first_block_info_pending_blocks<S>(
	storage_core_storage: &S,
	storage_core_state: OptionLink<Co>,
) -> Result<Option<BlockInfo>, anyhow::Error>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
{
	let block_structure_pending = query_core(CO_CORE_NAME_STORAGE)
		.with_default()
		.map(|storage_core| storage_core.block_structure_pending)
		.execute(storage_core_storage, storage_core_state)
		.await?;
	Ok(block_structure_pending
		.stream(storage_core_storage)
		.try_first()
		.await?
		.map(|(_cid, pending)| pending.info().clone()))
}
