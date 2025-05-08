use crate::library::{
	application_actor::{ApplicationActorMessage, CreateCoRequest},
	tauri_error::CoTauriError,
};
use co_actor::ActorHandle;
use co_sdk::CoId;
use uuid::Uuid;

#[tauri::command]
pub(crate) async fn create_co(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	creator_did: String,
	co_name: String,
	public: bool,
	co_id: Option<CoId>,
) -> Result<(), CoTauriError> {
	actor_handle.dispatch(ApplicationActorMessage::CreateCo(CreateCoRequest {
		creator_did,
		co_id: co_id.unwrap_or(Uuid::new_v4().to_string().into()),
		co_name,
		public,
	}))?;
	Ok(())
}
