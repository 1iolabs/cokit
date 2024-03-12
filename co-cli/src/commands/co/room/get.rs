use crate::cli::Cli;
use exitcode::ExitCode;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {}

pub async fn command(_cli: &Cli, _command: &Command) -> Result<ExitCode, anyhow::Error> {
	Ok(exitcode::OK)
}
