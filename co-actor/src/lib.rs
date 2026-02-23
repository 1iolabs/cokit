#[cfg(all(feature = "js", feature = "native"))]
compile_error!("Features 'js' and 'native' cannot be enabled together.");

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
mod task_spawner_local;
#[cfg(feature = "native")]
mod tokio_task_spawner;

pub use actor::{Actor, ActorError, ActorHandle, ActorInstance, ActorState};
pub use actor_local::{LocalActor, LocalActorInstance, LocalActorSpawner};
pub use epic::{
	ActionDispatch, Actions, BoxEpic, Epic, EpicExt, EpicRuntime, JoinEpic, MergeEpic, SwitchEpic, TracingEpic,
};
#[cfg(feature = "js")]
pub use js_local_task_spawner::JsLocalTaskSpawner;
#[cfg(feature = "js")]
pub use js_task_spawner::{TaskHandle, TaskSpawner};
pub use response::{
	Response, ResponseBackPressureStream, ResponseBackPressureStreamReceiver, ResponseReceiver, ResponseStream,
	ResponseStreamReceiver, ResponseStreams,
};
pub use state::Reducer;
pub use task_handle::TaskError;
pub use task_spawner_local::{LocalTaskHandle, LocalTaskSpawner};
#[cfg(feature = "native")]
pub use tokio_task_spawner::{TaskHandle, TaskSpawner};
