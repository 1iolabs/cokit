use crate::library::{
	application_actor::{ApplicationActorMessage, SessionId},
	tauri_error::CoTauriError,
};
use cid::Cid;
use co_actor::ActorHandle;
use ipld_core::ipld::Ipld;

#[tauri::command]
pub(crate) async fn resolve_cid(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	session_id: SessionId,
	cid: Cid,
) -> Result<Ipld, CoTauriError> {
	Ok(actor_handle
		.request(|r| ApplicationActorMessage::ResolveCid(session_id, cid, r))
		.await??)
}
