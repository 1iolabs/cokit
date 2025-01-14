use crate::library::{
	application_actor::{ApplicationActorMessage, GetActionsRequest, GetActionsResponse},
	tauri_error::CoTauriError,
};
use cid::Cid;
use co_actor::ActorHandle;
use co_sdk::CoId;
use std::collections::BTreeSet;

#[tauri::command]
pub(crate) async fn get_actions(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	co: CoId,
	heads: BTreeSet<Cid>,
	count: usize,
	until: Option<Cid>,
) -> Result<GetActionsResponse, CoTauriError> {
	Ok(actor_handle
		.request(|r| ApplicationActorMessage::GetActions(GetActionsRequest { co, heads, count, until }, r))
		.await??)
}
