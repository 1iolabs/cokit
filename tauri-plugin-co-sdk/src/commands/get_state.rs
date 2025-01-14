use crate::library::{application_actor::ApplicationActorMessage, tauri_error::CoTauriError};
use cid::Cid;
use co_actor::ActorHandle;
use co_sdk::CoId;
use std::collections::BTreeSet;

#[tauri::command]
pub(crate) async fn get_co_state(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	co: CoId,
) -> Result<(Option<Cid>, BTreeSet<Cid>), CoTauriError> {
	Ok(actor_handle.request(|r| ApplicationActorMessage::GetCoState(co, r)).await??)
}
