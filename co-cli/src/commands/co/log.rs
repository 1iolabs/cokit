use crate::{
	cli::Cli,
	library::{application::application, cat::cat_output},
};
use co_sdk::CoId;
use exitcode::ExitCode;
use futures::{pin_mut, StreamExt};

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// CO ID
	pub co: CoId,

	/// Entries to print.
	#[arg(short, default_value_t = 10)]
	pub count: usize,

	/// Entries to print.
	#[arg(default_value_t = 0)]
	pub skip: usize,
}

pub async fn command(cli: &Cli, command: &Command) -> Result<ExitCode, anyhow::Error> {
	let application = application(cli).await?;
	let (storage, stream) = application.co_log_entries(&command.co).await?;

	// stream
	let mut index = 0;
	let stream = stream.take(command.count).skip(command.skip);
	pin_mut!(stream);
	while let Some(entry) = stream.next().await {
		match entry {
			Ok(entry) => {
				// Event
				println!("head ({index}) {}", entry.cid());
				println!("{:?}", entry.entry());

				// payload
				cat_output(storage.clone(), entry.entry().payload, true).await?;
			},
			Err(err) => println!("head ({index}) error: {:?}", err),
		}
		index += 1;
	}

	// result
	Ok(exitcode::OK)
}
