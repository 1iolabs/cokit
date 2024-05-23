use crate::{CoCoreResolver, CoStorage, Reducer, ReducerChangedHandler};
use async_trait::async_trait;
use co_core_co::Co;
use co_primitives::OptionLink;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cloneable CoState token which will be updated by the reducer everytime it changes.
#[derive(Debug, Clone, Default)]
pub struct CoState {
	state: Arc<RwLock<OptionLink<Co>>>,
}
impl CoState {
	pub fn new(value: OptionLink<Co>) -> Self {
		Self { state: Arc::new(RwLock::new(value)) }
	}

	pub async fn read(&self) -> OptionLink<Co> {
		*self.state.read().await
	}

	pub async fn write(&self, state: OptionLink<Co>) {
		*self.state.write().await = state;
	}
}
impl From<OptionLink<Co>> for CoState {
	fn from(value: OptionLink<Co>) -> Self {
		Self::new(value)
	}
}
#[async_trait]
impl ReducerChangedHandler<CoStorage, CoCoreResolver> for CoState {
	async fn on_state_changed(&mut self, reducer: &Reducer<CoStorage, CoCoreResolver>) -> Result<(), anyhow::Error> {
		self.write(reducer.state().into()).await;
		Ok(())
	}
}
