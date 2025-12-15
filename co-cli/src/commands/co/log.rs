use crate::{
	cli::Cli,
	library::{
		cat::{cat_output, CatOptions},
		cli_context::CliContext,
	},
};
use co_sdk::{BlockStorageContentMapping, CoId, MultiCodec};
use exitcode::ExitCode;
use futures::{pin_mut, StreamExt};

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// CO ID
	pub co: CoId,

	/// Entries to print.
	#[arg(short('n'), long, default_value_t = 10)]
	pub count: usize,

	/// Entries to print.
	#[arg(short, long, default_value_t = 0)]
	pub skip: usize,
}

pub async fn command(context: &CliContext, cli: &Cli, command: &Command) -> Result<ExitCode, anyhow::Error> {
	let application = context.application(cli).await;
	let (storage, stream) = application.co().entries(&command.co).await?;

	// stream
	let mut index = 0;
	let stream = stream.take(command.count).skip(command.skip);
	pin_mut!(stream);
	while let Some(entry) = stream.next().await {
		match entry {
			Ok(entry) => {
				// event
				print!("head ({index}) {}", entry.cid());
				if storage.is_content_mapped().await {
					let encrypted_cid = storage.to_plain(entry.cid()).await;
					if let Some(encrypted_cid) = encrypted_cid {
						print!(" ({}: {})", MultiCodec::from(encrypted_cid.codec()), encrypted_cid);
					} else {
						print!(" (no mapping)");
					}
				}
				println!();
				println!("{:?}", entry.entry());

				// payload
				cat_output(
					storage.clone(),
					entry.entry().payload,
					CatOptions::default().with_pretty(true).with_decrypt(true),
				)
				.await?;
				println!();
			},
			Err(err) => println!("head ({index}) error: {:?}", err),
		}
		index += 1;
	}

	// result
	Ok(exitcode::OK)
}
