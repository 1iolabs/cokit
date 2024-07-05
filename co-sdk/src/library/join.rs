use crate::{CoContext, CoReducerFactory, CO_CORE_NAME_CO};
use anyhow::anyhow;
use co_core_co::{Co, CoAction, ParticipantState};
use co_identity::{DidCommHeader, Identity, Message, PrivateIdentity};
use co_network::didcomm::EncodedMessage;
use co_primitives::{CoId, Did, Tags};
use serde::{Deserialize, Serialize};

pub const CO_DIDCOMM_JOIN: &str = "co-join";

/// Create an encoded join message.
pub fn create_join_message<F, T>(from: &F, to: &T, co: CoId, thid: Option<String>) -> anyhow::Result<EncodedMessage>
where
	F: PrivateIdentity + Send + Sync + 'static,
	T: Identity + Send + Sync + 'static,
{
	let (from_didcomm, to_didcomm, mut header) = DidCommHeader::create(from, to, CO_DIDCOMM_JOIN)?;
	header.thid = thid;
	let body = serde_json::to_string(&co)?;
	let message = from_didcomm.jwe(&to_didcomm, header, &body)?;
	Ok(EncodedMessage(message.into_bytes()))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoJoinPayload {
	pub id: CoId,
}

pub fn is_join_message(message: Message) -> bool {
	message.header().message_type == CO_DIDCOMM_JOIN
}

/// Join a participant when we receive a join message from a remote.
pub async fn join<P>(
	co_context: CoContext,
	identity: &P,
	message: Message,
	participant_tags: Tags,
) -> anyhow::Result<()>
where
	P: PrivateIdentity + Send + Sync + 'static,
{
	let sender = message.sender().ok_or(anyhow!("Expected validated sender"))?;
	let body: CoJoinPayload = message.body_deserialize()?;
	let co = co_context
		.co_reducer(&body.id)
		.await?
		.ok_or(anyhow!("Unknown CO: {}", body.id))?;
	let co_core = co.co().await?;
	let action = join_participant_state(&co_core, sender, participant_tags).ok_or(anyhow!("Not allowed to join"))?;
	co.push(identity, CO_CORE_NAME_CO, &action).await?;
	Ok(())
}

fn join_participant_state(co: &Co, did: &Did, participant_tags: Tags) -> Option<CoAction> {
	match get_join_setting(&co.tags) {
		JoinSetting::Invite => co
			.participants
			.get(did)
			.filter(|participant| participant.state == ParticipantState::Invite)
			.map(|_| CoAction::ParticipantJoin { participant: did.to_owned(), tags: participant_tags }),
		JoinSetting::All => Some(CoAction::ParticipantJoin { participant: did.to_owned(), tags: participant_tags }),
		JoinSetting::Did => None, // TODO: implement
		JoinSetting::Manual => {
			Some(CoAction::ParticipantPending { participant: did.to_owned(), tags: participant_tags })
		},
	}
}

#[derive(Debug)]
enum JoinSetting {
	Invite,
	All,
	Did,
	Manual,
}
fn get_join_setting(tags: &Tags) -> JoinSetting {
	match tags.string("co-join").unwrap_or("") {
		"all" => JoinSetting::All,
		"did" => JoinSetting::Did,
		"manual" => JoinSetting::Manual,
		_ => JoinSetting::Invite,
	}
}
