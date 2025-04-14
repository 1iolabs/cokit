use super::Command as DidCommand;
use crate::{cli::Cli, library::cli_context::CliContext};
use co_core_co::{CoAction, ParticipantState};
use co_sdk::{
	find_co_private_identity, Action, CoId, CoReducerFactory, Did, Identity, PrivateIdentityResolver, CO_CORE_NAME_CO,
};
use exitcode::ExitCode;
use futures::StreamExt;
use std::future::ready;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// CO ID
	pub co: CoId,

	/// Participant DID to be invited.
	pub did: Did,

	/// Participant DID of the sender of the invite.
	#[arg(short, long)]
	pub from: Option<Did>,
}

pub async fn command(
	context: &CliContext,
	cli: &Cli,
	_did_command: &DidCommand,
	command: &Command,
) -> Result<ExitCode, anyhow::Error> {
	let mut application = context.application(cli).await;
	application.create_network(false).await?;
	let co_reducer = application.context().try_co_reducer(&command.co).await?;
	let (_storage, co) = co_reducer.co().await?;
	let participant = co.participants.get(&command.did);
	let only_network = if let Some(participant) = participant {
		match participant.state {
			ParticipantState::Active => {
				println!("Participant is alerday active.");
				return Ok(exitcode::DATAERR);
			},
			ParticipantState::Invite => Some(participant.did.clone()),
			_ => None,
		}
	} else {
		None
	};

	// from
	let from = match &command.from {
		Some(did) => {
			let resolver = application.context().private_identity_resolver().await?;
			resolver.resolve_private(did).await?
		},
		None => find_co_private_identity(application.context(), &command.co).await?,
	};

	// result
	let done = tokio::spawn({
		let command_co = co_reducer.id().clone();
		let command_did = command.did.clone();
		let actions = application.actions();
		async move {
			actions
				.filter_map(|action| {
					ready(match &action {
						Action::InviteSent { co, participant, peer }
							if co == &command_co && participant == &command_did =>
						{
							println!("invited: {:?}", peer);
							Some(())
						},
						Action::Error { err } => {
							eprintln!("error: {:?}", err);
							Some(())
						},
						_ => None,
					})
				})
				.take(1)
				.collect::<Vec<_>>()
				.await;
		}
	});

	// resend/invite
	if let Some(to) = only_network {
		application.handle().dispatch(Action::Invite {
			co: co_reducer.id().clone(),
			from: from.identity().to_owned(),
			to,
		})?;
	} else {
		let action = CoAction::ParticipantInvite { participant: command.did.clone(), tags: Default::default() };
		co_reducer.push(&from, CO_CORE_NAME_CO, &action).await?;
	}

	// wait invite or error
	done.await?;

	// result
	Ok(exitcode::OK)
}
