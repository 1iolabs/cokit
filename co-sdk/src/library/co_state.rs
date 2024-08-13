use crate::{
	reducer::core_resolver::dynamic::DynamicCoreResolver, CoStorage, Reducer, ReducerChangeContext,
	ReducerChangedHandler,
};
use async_trait::async_trait;
use co_core_co::Co;
use co_primitives::OptionLink;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cloneable CoState token which will be updated by the reducer everytime it changes.
#[derive(Debug, Clone, Default)]
pub struct CoState {
	state: Arc<RwLock<(Option<CoStorage>, OptionLink<Co>)>>,
}
impl CoState {
	pub fn new(storage: Option<CoStorage>, value: OptionLink<Co>) -> Self {
		Self { state: Arc::new(RwLock::new((storage, value))) }
	}

	/// Write value and optionally store an clone of stage if not set yet.
	pub async fn write(&self, storage: &CoStorage, value: OptionLink<Co>, force_update_storage: bool) {
		let mut guard = self.state.write().await;
		if guard.0.is_none() || force_update_storage {
			*guard = (Some(storage.clone()), value);
		} else {
			guard.1 = value;
		}
	}

	pub async fn read(&self) -> (Option<CoStorage>, OptionLink<Co>) {
		self.state.read().await.clone()
	}

	// pub async fn read_state(&self) -> OptionLink<Co> {
	// 	self.state.read().await.1
	// }

	// pub async fn write_state(&self, state: OptionLink<Co>) {
	// 	self.state.write().await.1 = state;
	// }
}
impl From<OptionLink<Co>> for CoState {
	fn from(value: OptionLink<Co>) -> Self {
		Self::new(None, value)
	}
}
#[async_trait]
impl ReducerChangedHandler<CoStorage, DynamicCoreResolver<CoStorage>> for CoState {
	async fn on_state_changed(
		&mut self,
		reducer: &Reducer<CoStorage, DynamicCoreResolver<CoStorage>>,
		_context: ReducerChangeContext,
	) -> Result<(), anyhow::Error> {
		self.write(reducer.log().storage(), reducer.state().into(), false).await;
		Ok(())
	}
}
