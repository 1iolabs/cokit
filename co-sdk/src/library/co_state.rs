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
#[deprecated]
pub struct CoState {
	state: Arc<RwLock<OptionLink<Co>>>,
}
impl CoState {
	pub fn new(value: OptionLink<Co>) -> Self {
		Self { state: Arc::new(RwLock::new(value)) }
	}

	pub async fn read_state(&self) -> OptionLink<Co> {
		*self.state.read().await
	}

	pub async fn write_state(&self, state: OptionLink<Co>) {
		*self.state.write().await = state;
	}
}
impl From<OptionLink<Co>> for CoState {
	fn from(value: OptionLink<Co>) -> Self {
		Self::new(value)
	}
}
#[async_trait]
impl ReducerChangedHandler<CoStorage, DynamicCoreResolver<CoStorage>> for CoState {
	async fn on_state_changed(
		&mut self,
		reducer: &Reducer<CoStorage, DynamicCoreResolver<CoStorage>>,
		_context: ReducerChangeContext,
	) -> Result<(), anyhow::Error> {
		self.write_state(reducer.state().into()).await;
		Ok(())
	}
}
