use crate::library::{application_actor::ApplicationActorMessage, tauri_error::CoTauriError};
use co_actor::ActorHandle;
use co_sdk::CoId;
use libipld::{Cid, Ipld};

#[tauri::command]
pub(crate) async fn resolve_cid(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	co: CoId,
	cid: Cid,
) -> Result<Ipld, CoTauriError> {
	Ok(actor_handle
		.request(|r| ApplicationActorMessage::ResolveCid(co, cid, r))
		.await??)
}
