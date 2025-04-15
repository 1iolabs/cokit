use crate::{CoReducer, CoStorage};
use async_trait::async_trait;
use std::{fmt::Debug, sync::Arc};

#[async_trait]
pub trait CoReducerContext: Debug {
	/// Get a new storage instance.
	///
	/// # Args
	/// - `force_local` - If true the new instance should not use networking.
	fn storage(&self, force_local: bool) -> CoStorage;

	/// Refresh reducer instance state from source.
	async fn refresh(&self, parent: CoReducer, co: CoReducer) -> anyhow::Result<()>;

	/// Clear reducer caches.
	async fn clear(&self, co: CoReducer);
}

pub type CoReducerContextRef = Arc<dyn CoReducerContext + Send + Sync + 'static>;
