use super::co_application::{application, CoApplicationSettings};
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, Response};
use co_sdk::{Application, CoId, Tags};
use libipld::{Block, Cid, DefaultParams};

pub struct TauriContext {}
pub struct TauriContextState {
	application: Application,
}
pub enum TauriContextMessage {
	StorageGet(CoId, Cid, Response<Block<DefaultParams>>),
	StorageSet(),
}

#[async_trait]
impl Actor for TauriContext {
	type Message = TauriContextMessage;

	type State = TauriContextState;

	type Initialize = CoApplicationSettings;

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: Tags,
		initialize: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		Ok(TauriContextState { application: application(initialize).await })
	}

	async fn handle(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_message: Self::Message,
		_state: &mut Self::State,
	) -> Result<(), ActorError> {
		Ok(())
	}
}
