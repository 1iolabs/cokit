// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::library::application_actor::{ApplicationActorMessage, SessionId};
use anyhow::anyhow;
use co_actor::ActorHandle;
use co_sdk::{from_cbor, to_cbor, Did};
use ipld_core::ipld::Ipld;
use serde::Deserialize;
use std::collections::BTreeMap;
use tauri::ipc::{InvokeError, Response};

/// This is the data structure of the single argument body of the push command. Argument is given as raw data (Vec<u8>)
/// and then deserialized into this data type. When a tauri command is given a single parameter as Vec<u8>,
/// tauri skips serialization/deserialization.  We want to ser/de by hand using cbor so possible Cids given in the
/// action won't get broken by json serialization. We also don't know what type of action is given yet, so we just
/// deserialize it into Ipld type. Using Ipld types as parameters directly doesn't work well as it doesn't
/// deserialize Cids correctly.
#[derive(Deserialize, Debug)]
struct PushCommandBody {
	session: SessionId,
	core: String,
	action: Ipld,
	identity: Did,
}

impl TryFrom<Ipld> for PushCommandBody {
	type Error = anyhow::Error;
	fn try_from(value: Ipld) -> Result<Self, Self::Error> {
		// convert Ipld data structure into this type
		match value {
			Ipld::Map(map) => {
				let action = PushCommandBody::resolve_action(&map)?;
				let session = PushCommandBody::resolve_session_id(&map)?;
				let core = PushCommandBody::resolve_core(&map)?;
				let identity = PushCommandBody::resolve_identity(&map)?;
				Ok(PushCommandBody { action, session, core, identity })
			},
			_ => Err(anyhow!("Ipld is not a map")),
		}
	}
}

impl PushCommandBody {
	fn resolve_action(map: &BTreeMap<String, Ipld>) -> Result<Ipld, anyhow::Error> {
		if let Some(action) = map.get("action") {
			Ok(action.clone())
		} else {
			Err(anyhow!("Body contains no action"))
		}
	}
	fn resolve_session_id(map: &BTreeMap<String, Ipld>) -> Result<SessionId, anyhow::Error> {
		if let Some(ipld) = map.get("session") {
			match ipld {
				Ipld::String(session) => Ok(session.into()),
				_ => Err(anyhow!("Session is not a string")),
			}
		} else {
			Err(anyhow!("Body contains no session info"))
		}
	}
	fn resolve_core(map: &BTreeMap<String, Ipld>) -> Result<String, anyhow::Error> {
		if let Some(ipld) = map.get("core") {
			match ipld {
				Ipld::String(core) => Ok(core.clone()),
				_ => Err(anyhow!("core not a string")),
			}
		} else {
			Err(anyhow!("body contains no core info"))
		}
	}
	fn resolve_identity(map: &BTreeMap<String, Ipld>) -> Result<String, anyhow::Error> {
		if let Some(ipld) = map.get("identity") {
			match ipld {
				Ipld::String(identity) => Ok(identity.clone()),
				_ => Err(anyhow!("Identity not a string")),
			}
		} else {
			Err(anyhow!("body contains no identity info"))
		}
	}
}

#[tauri::command]
pub async fn push_action(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	request: tauri::ipc::Request<'_>,
) -> Result<Response, InvokeError> {
	// manually deserialize body into PushCommandBody type
	let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
		return Err(InvokeError::from_anyhow(anyhow!("Request body must be raw")));
	};
	let body: PushCommandBody = from_cbor(bytes).map_err(InvokeError::from_error)?;
	tracing::info!(
		"tauri command push: \n\tSession: {:#?}\n\tcore: {:#?}\n\taction: {:#?}",
		body.session,
		body.core,
		body.action
	);
	let cid = actor_handle
		.request(|r| ApplicationActorMessage::Push(body.session, body.core, body.action, body.identity, r))
		.await
		.map_err(InvokeError::from_error)?
		.map_err(InvokeError::from_anyhow)?;
	Ok(Response::new(to_cbor(&cid).map_err(InvokeError::from_error)?))
}
