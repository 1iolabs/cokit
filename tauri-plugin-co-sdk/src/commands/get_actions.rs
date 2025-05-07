use crate::library::{
	application_actor::{ApplicationActorMessage, GetActionsRequest, GetActionsResponse, SessionId},
	tauri_error::CoTauriError,
};
use cid::Cid;
use co_actor::ActorHandle;
use std::collections::BTreeSet;

#[tauri::command]
pub(crate) async fn get_actions(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	session_id: SessionId,
	heads: BTreeSet<Cid>,
	count: usize,
	until: Option<Cid>,
) -> Result<GetActionsResponse, CoTauriError> {
	Ok(actor_handle
		.request(|r| ApplicationActorMessage::GetActions(GetActionsRequest { session_id, heads, count, until }, r))
		.await??)
}
