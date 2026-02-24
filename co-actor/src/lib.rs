// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

mod actor;
mod actor_local;
mod epic;
mod response;
mod state;
mod task_spawner;
mod task_spawner_local;

pub use actor::{Actor, ActorError, ActorHandle, ActorInstance, ActorState};
pub use actor_local::{LocalActor, LocalActorInstance, LocalActorSpawner};
pub use epic::{
	ActionDispatch, Actions, BoxEpic, Epic, EpicExt, EpicRuntime, JoinEpic, MergeEpic, SwitchEpic, TracingEpic,
};
pub use response::{
	Response, ResponseBackPressureStream, ResponseBackPressureStreamReceiver, ResponseReceiver, ResponseStream,
	ResponseStreamReceiver, ResponseStreams,
};
pub use state::Reducer;
pub use task_spawner::TaskSpawner;
pub use task_spawner_local::{LocalJoinError, LocalJoinHandle, LocalTaskSpawner};
