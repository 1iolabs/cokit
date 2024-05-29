use crate::didcomm::EncodedMessage;
use co_identity::DidCommHeader;
use co_primitives::CoId;
use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::{
	collections::BTreeSet,
	time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HeadsMessage {
	#[serde(rename = "h")]
	Heads(CoId, BTreeSet<Cid>),
}
impl HeadsMessage {
	/// Encode as DIDComm message.
	pub fn to_didcomm(&self) -> Result<EncodedMessage, anyhow::Error> {
		let time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
		let header = DidCommHeader {
			created_time: Some(time),
			expires_time: Some(time + 120),
			from: None,
			id: Uuid::new_v4().into(),
			message_type: format!("co-heads/1.0.0"),
			pthid: None,
			thid: None,
			to: Default::default(),
		};
		Ok(EncodedMessage::create_plain_json(header, self)?)
	}
}
