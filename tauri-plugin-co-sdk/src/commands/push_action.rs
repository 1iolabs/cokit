use crate::library::{application_actor::ApplicationActorMessage, tauri_error::CoTauriError};
use anyhow::anyhow;
use co_actor::ActorHandle;
use co_sdk::{CoId, Did};
use libipld::{cbor::DagCborCodec, codec::Codec, Cid, Ipld};
use serde::Deserialize;
use std::collections::BTreeMap;

/// This is the data structure of the single argument body of the push command. Argument is given as raw data (Vec<u8>)
/// and then deserialized into this data type. When a tauri command is given a single parameter as Vec<u8>,
/// tauri skips serialization/deserialization.  We want to ser/de by hand using cbor so possible Cids given in the
/// action wont get broken by json serialization. We also don't know what type of action is given yet, so we just
/// deserialize it into Ipld type. Using Ipld types as parameters directly doesn't work well as it doesn't
/// deserialize Cids correctly.
#[derive(Deserialize, Debug)]
struct PushCommandBody {
	co: CoId,
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
				let co = PushCommandBody::resolve_co_id(&map)?;
				let core = PushCommandBody::resolve_core(&map)?;
				let identity = PushCommandBody::resolve_identity(&map)?;
				Ok(PushCommandBody { action, co, core, identity })
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
	fn resolve_co_id(map: &BTreeMap<String, Ipld>) -> Result<CoId, anyhow::Error> {
		if let Some(ipld) = map.get("co") {
			match ipld {
				Ipld::String(co) => Ok(CoId::new(&*co)),
				_ => Err(anyhow!("Co is not a string")),
			}
		} else {
			Err(anyhow!("Body contains no co info"))
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
	body: Vec<u8>,
) -> Result<Option<Cid>, CoTauriError> {
	// manually deserialize body into PushCommandBody type
	let body: PushCommandBody = DagCborCodec::default().decode::<Ipld>(&body)?.try_into()?;
	tracing::info!(
		"tauri command push: \n\tCo: {:#?}\n\tcore: {:#?}\n\taction: {:#?}",
		body.co,
		body.core,
		body.action
	);
	Ok(actor_handle
		.request(|r| ApplicationActorMessage::Push(body.co, body.core, body.action, body.identity, r))
		.await??)
}
