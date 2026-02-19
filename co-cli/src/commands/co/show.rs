// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	cli::Cli,
	library::{
		cat::{cat_output, CatOptions},
		cli_context::CliContext,
	},
};
use co_sdk::{BlockStorageExt, CoId, CoReducerFactory, CoStorage, OptionLink};
use exitcode::ExitCode;
use futures::{StreamExt, TryStreamExt};

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// CO ID
	pub co: CoId,

	/// Inspect core with name.
	#[arg(short, long)]
	pub core: Option<String>,
}

pub async fn command(context: &CliContext, cli: &Cli, command: &Command) -> Result<ExitCode, anyhow::Error> {
	let application = context.application(cli).await;
	let reducer = application.context().try_co_reducer(&command.co).await?;
	let reducer_state = reducer.reducer_state().await;
	let (storage, co) = reducer.co().await?;
	if let Some(core_name) = &command.core {
		// core
		let core = co.cores.get(core_name).ok_or(anyhow::anyhow!("Core not found: {core_name}"))?;
		println!("# Core {}", core_name);
		println!("- State: `{:?}`", core.state);
		println!("- Tags: {:?}", core.tags);
		println!();
		match core.tags.string("core") {
			Some("co-core-storage") => {
				print_storage(&storage, core.state.into()).await?;
			},
			_ => {
				if let Some(core_state) = core.state {
					println!("## State");
					println!("```");
					cat_output(storage, core_state, CatOptions::default().with_pretty(true).with_decrypt(true)).await?;
					println!("```");
				}
			},
		}
	} else {
		// co
		println!("# CO");
		println!("- Name: {}", co.name);
		println!("- Tags: {}", co.tags);
		println!("- State: `{:?}`", reducer_state.state());
		println!("- Heads: `{:?}`", reducer_state.heads());
		println!();

		// participants
		let participants = co.participants.stream(&storage).try_collect::<Vec<_>>().await?;
		println!("# Participants ({})", participants.len());
		for participant in participants {
			println!("## {}", participant.1.did,);
			println!("- Status: `{:?}`", participant.1.state);
			println!("- Tags: {:?}", participant.1.tags);
		}
		println!();

		// cores
		let cores = co.cores;
		println!("# Cores ({})", cores.len());
		for core in cores {
			println!("## {}", core.0);
			println!("- State: `{:?}`", core.1.state);
			println!("- Tags: {:?}", core.1.tags);
		}
		println!();
	}

	// result
	Ok(exitcode::OK)
}

async fn print_storage(storage: &CoStorage, state: OptionLink<co_core_storage::Storage>) -> Result<(), anyhow::Error> {
	println!("## co-core-storage");
	let storage_state = storage.get_value(&state).await?;

	// pins
	println!("### Pins");
	storage_state
		.pins
		.stream(storage)
		.try_for_each({
			let storage = storage.clone();
			move |(name, pin)| {
				let storage = storage.clone();
				async move {
					println!("#### {name}");
					println!("- References: {}", pin.references_count);
					println!("- Strategy: {:?}", pin.strategy);
					println!();
					println!("Pins:");
					let storage = storage.clone();
					pin.references
						.stream(&storage)
						.enumerate()
						.map(|(index, item)| item.map(|inner| (index, inner)))
						.try_for_each({
							move |(index, (_list_index, item))| async move {
								println!("{}. {}", index, item.cid());
								Ok(())
							}
						})
						.await?;
					Ok(())
				}
			}
		})
		.await?;
	println!();

	// blocks
	println!("### Blocks");
	storage_state
		.blocks
		.stream(storage)
		.try_for_each({
			move |(block_cid, block)| async move {
				println!("#### {}{}", block_cid.cid(), if block.is_removable() { " (removable)" } else { "" });
				println!("- References: {}", block.references);
				println!("- Mode: {:?}", block.mode);
				println!("- Tags: {}", block.tags);
				println!();
				Ok(())
			}
		})
		.await?;

	Ok(())
}
