use crate::{cli::Cli, library::cli_context::CliContext};
use anyhow::anyhow;
use cid::Cid;
use co_primitives::{from_cbor, CoId, DagCollectionAsyncExt};
use co_runtime::{create_cid_resolver, MultiLayerCidResolver};
use co_sdk::{
	state::{memberships, query_core, QueryExt},
	Application, CoReducerFactory, CoStorage, CO_CORE_NAME_PIN, CO_ID_LOCAL,
};
use exitcode::ExitCode;
use futures::{pin_mut, StreamExt, TryStreamExt};
use std::{
	collections::{BTreeMap, BTreeSet},
	fmt::Debug,
};
use tokio::fs;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// Pin testing commands
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Commands {
	/// Lists cids that are manually pinned
	Ls(ListCommand),
	/// Generates pins by traversing state
	Gen(GenerateCommand),
	/// Updates the pin map for auto state pinning
	Update(UpdateCommand),
}

#[derive(Debug, Clone, clap::Args)]
pub struct ListCommand {
	/// Sums up the number of pins
	#[arg(short, long, default_value_t = false)]
	pub sum: bool,
	/// Includes tags when printing
	#[arg(short, long, default_value_t = false)]
	pub all: bool,
	/// Lists all pins
	#[arg(short, long, default_value_t = false)]
	pub list: bool,
}

#[derive(Debug, Clone, clap::Args)]
pub struct GenerateCommand {
	/// depth to generate pinned cids to
	#[arg(short, long, default_value_t = -1)]
	pub depth: i64,
	/// co id
	#[arg(short, long, default_value_t = CoId::new(CO_ID_LOCAL))]
	pub co: CoId,
}

#[derive(Debug, Clone, clap::Args)]
pub struct UpdateCommand {}

pub async fn command(context: &CliContext, cli: &Cli, command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &command.command {
		Commands::Ls(list_command) => list_pins(context, cli, list_command).await,
		Commands::Gen(gen_command) => generate_pins(context, cli, gen_command).await,
		Commands::Update(update_command) => update_pins(context, cli, update_command).await,
	}
}

/// list function for all manual pins
pub async fn list_pins(context: &CliContext, cli: &Cli, command: &ListCommand) -> Result<ExitCode, anyhow::Error> {
	let application = context.application(cli).await;

	let local_co_reducer = application.local_co_reducer().await?;
	let (storage, pin_state) = query_core::<co_core_pin::Pin>(CO_CORE_NAME_PIN)
		.execute_reducer(&local_co_reducer)
		.await?;
	let pins = pin_state.pins.stream(&storage);
	let inner: Vec<_> = pins.try_collect().await?;
	if command.sum {
		println!("Total number of current pins: {}", inner.len());
	}
	if command.sum && command.list {
		// get terminal width
		let (x, _y) = termion::terminal_size().unwrap();
		// hline
		println!("{:-<width$}", "-", width = x as usize);
	}
	if command.list {
		for (cid, tags) in inner.iter() {
			if command.all {
				let tags: Vec<_> = tags.stream(&storage).try_collect().await?;
				println!("Cid {} pinned by tags:\n\t {:?}", cid, tags);
			} else {
				println!("{}", cid);
			}
		}
	}
	Ok(exitcode::OK)
}

pub async fn generate_pins(
	context: &CliContext,
	cli: &Cli,
	command: &GenerateCommand,
) -> Result<ExitCode, anyhow::Error> {
	// get state of given co
	let application = context.application(cli).await;
	let co_reducer = application.context().try_co_reducer(&command.co).await?;
	let state = co_reducer.reducer_state().await.0;

	if let Some(state) = state {
		// generate cids up to depth
		let resolver = &create_cid_resolver(get_all_co_storages(application).await?).await?;
		let result = MultiLayerCidResolver::new()
			.with_depth_limit(command.depth)
			.resolve_cid(&state, resolver)
			.await;

		// print findings
		result.print_results();
	}
	Ok(exitcode::OK)
}

async fn update_pins(context: &CliContext, cli: &Cli, _command: &UpdateCommand) -> Result<ExitCode, anyhow::Error> {
	// application ini
	let application = context.application(cli).await;
	// get pinn file path
	let pins_path = application
		.settings()
		.application_path
		.as_ref()
		.ok_or(anyhow!("expeced filesystem application"))?
		.with_file_name("pins.cbor");
	// read previous pins from file
	let content = fs::read(&pins_path).await?;
	// decode cbor
	let old_pin_map: BTreeMap<Cid, BTreeSet<Cid>> = from_cbor(&content)?;

	// local co state
	let state = application.local_co_reducer().await?.reducer_state().await.state();

	// create resolver
	let resolver = create_cid_resolver(get_all_co_storages(application).await?).await?;
	let resolver_result = MultiLayerCidResolver::new()
		.with_previous_cids(old_pin_map)
		.resolve_cid(&state.unwrap(), &resolver)
		.await;

	// write pin map
	let data = serde_ipld_dagcbor::to_vec(&resolver_result.new_cid_map)?;
	fs::write(&pins_path, data).await?;

	resolver_result.print_diff();

	Ok(exitcode::OK)
}

async fn get_all_co_storages(application: Application) -> anyhow::Result<Vec<CoStorage>> {
	let local_co_reducer = application.local_co_reducer().await?;
	let stream = memberships(local_co_reducer.storage(), local_co_reducer.reducer_state().await.co());
	let mut storages: Vec<CoStorage> = vec![];
	pin_mut!(stream);
	while let Some(result) = stream.next().await {
		match result {
			Ok((co, _, _, _)) => {
				if let Some(reducer) = application.co_reducer(co).await? {
					storages.push(reducer.storage());
				}
			},
			Err(_) => (),
		}
	}
	Ok(storages)
}
