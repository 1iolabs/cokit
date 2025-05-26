use crate::{cli::Cli, library::cli_context::CliContext};
use anyhow::anyhow;
use co_core_co::Co;
use co_core_storage::BlockInfo;
use co_primitives::{CoTryStreamExt, WeakCoReferenceFilter};
use co_sdk::{
	state::{query_core, Query, QueryExt},
	storage_cleanup, storage_structure_recursive, BlockStorageContentMapping, CoId, CoPinningKey, CoReducerFactory,
	CoStructureResolver, OptionLink, CO_CORE_NAME_STORAGE,
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
		let first_info = first_block_info_shallow_blocks(&local_storage, local_co.reducer_state().await.co()).await?;
		if let Some(first_info) = first_info {
			if let Some(first_pin) = first_info.pins.stream(&local_storage).try_first().await? {
				// verify we do not loop twice
				if last_pin.as_ref() == Some(&first_pin) {
					return Err(anyhow!("Found pin after remove: {}", first_pin));
				}
				last_pin = Some(first_pin.clone());

				// resolve the co for the pin
				let (_pinning_key, co_id) = parse_co_id_from_pin(first_pin)?;
				let co = application.co().try_co_reducer(&co_id).await?;
				let co_storage = co.storage();

				// output
				if !cli.quiet {
					println!("checking {} ...", co_id.as_str());
				}

				// resolve
				storage_structure_recursive(
					&local_storage,
					&mut local_dispatcher,
					local_co.reducer_state().await.co(),
					&co_storage,
					None,
					&CoStructureResolver::new(
						&co_id,
						application.co().block_links().clone().with_filter(WeakCoReferenceFilter::new()),
					),
				)
				.await?;

				// try next
				continue;
			}
		}
		break;
	}

	// remove
	if !cli.quiet {
		println!("cleanup");
	}
	storage_cleanup(&mut local_dispatcher, &local_storage, local_co.reducer_state().await.co()).await?;

	// result
	Ok(exitcode::OK)
}

fn parse_co_id_from_pin(mut pin: String) -> Result<(CoPinningKey, CoId), anyhow::Error> {
	if pin.starts_with("co.state.") {
		Ok((CoPinningKey::State, pin.split_off("co.state.".len()).into()))
	} else if pin.starts_with("co.log.") {
		Ok((CoPinningKey::Log, pin.split_off("co.log.".len()).into()))
	} else {
		Err(anyhow!("Parse pin failed: {}", pin))
	}
}

async fn first_block_info_shallow_blocks<S>(
	storage_core_storage: &S,
	storage_core_state: OptionLink<Co>,
) -> Result<Option<BlockInfo>, anyhow::Error>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
{
	// get shallow references
	let mut query_blocks_index_shallow = query_core::<co_core_storage::Storage>(CO_CORE_NAME_STORAGE)
		.with_default()
		.map(|storage_core| storage_core.blocks_index_shallow);
	let blocks_index_shallow = query_blocks_index_shallow
		.execute(storage_core_storage, storage_core_state)
		.await?;
	Ok(blocks_index_shallow
		.stream(storage_core_storage)
		.try_first()
		.await?
		.map(|(_cid, info)| info))
}
