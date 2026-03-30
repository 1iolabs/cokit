// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

// modules
mod actor;
mod actor_local;
mod epic;
#[cfg(feature = "js")]
mod js_local_task_spawner;
#[cfg(feature = "js")]
mod js_task_spawner;
mod response;
mod state;
mod task_handle;
mod task_options;
mod task_spawner_local;
pub mod time;
#[cfg(not(feature = "js"))]
mod tokio_task_spawner;

// exports
pub use actor::{Actor, ActorError, ActorHandle, ActorInstance, ActorSpawner, ActorState};
pub use actor_local::{LocalActor, LocalActorInstance, LocalActorSpawner};
pub use backend::{TaskHandle, TaskSpawner};
pub use epic::{
	ActionDispatch, Actions, BoxEpic, Epic, EpicExt, EpicRuntime, JoinEpic, MergeEpic, SwitchEpic, TracingEpic,
};
#[cfg(feature = "js")]
pub use js::JsLocalTaskSpawner;
pub use response::{
	Response, ResponseBackPressureStream, ResponseBackPressureStreamReceiver, ResponseReceiver, ResponseStream,
	ResponseStreamReceiver, ResponseStreams,
};
pub use state::Reducer;
pub use task_handle::TaskError;
pub use task_options::TaskOptions;
pub use task_spawner_local::{LocalTaskHandle, LocalTaskSpawner};

// backends
#[cfg(not(feature = "js"))]
mod backend {
	pub use super::tokio_task_spawner::{TaskHandle, TaskSpawner};
}
#[cfg(not(feature = "js"))]
pub mod tokio {
	pub use super::tokio_task_spawner::{TaskHandle, TaskSpawner};
}
#[cfg(feature = "js")]
mod backend {
	pub use super::js_task_spawner::{TaskHandle, TaskSpawner};
}
#[cfg(feature = "js")]
pub mod js {
	pub use super::{
		js_local_task_spawner::JsLocalTaskSpawner,
		js_task_spawner::{TaskHandle, TaskSpawner},
	};
}
