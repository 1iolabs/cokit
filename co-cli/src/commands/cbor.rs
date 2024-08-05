use crate::library::cli_context::CliContext;
use co_primitives::from_cbor;
use exitcode::ExitCode;
use libipld::Ipld;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// CBOR Command
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Commands {
	/// Print cbor from file.
	Print(PrintCommand),
}

#[derive(Debug, Clone, clap::Args)]
pub struct PrintCommand {
	/// The file to print.
	pub file: String,

	/// Pretty print data.
	#[arg(short, long)]
	pub pretty: bool,
}

pub async fn command(_context: &CliContext, cbor: &Command) -> Result<ExitCode, anyhow::Error> {
	match &cbor.command {
		Commands::Print(command) => print(command).await,
	}
}

async fn print(file: &PrintCommand) -> Result<ExitCode, anyhow::Error> {
	let buf: Vec<u8> = tokio::fs::read(&file.file).await?;
	let ipld: Ipld = from_cbor(&buf)?;
	if file.pretty {
		println!("{:#?}", ipld);
	} else {
		println!("{:?}", ipld);
	}
	Ok(exitcode::OK)
}
