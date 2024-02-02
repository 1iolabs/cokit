use async_trait::async_trait;
use co_runtime::Core;
use co_storage::StorageError;
use libipld::Cid;

#[async_trait]
pub trait CoreResolver {
	/// Resolve the COre responsible for reducing the action.
	async fn resolve_core(&self, action: &Cid) -> Result<Core, StorageError>;
}

#[derive(Debug, Clone)]
pub struct SingleCoreResolver {
	core: Core,
}
impl SingleCoreResolver {
	pub fn new(core: Core) -> Self {
		Self { core }
	}
}
#[async_trait]
impl CoreResolver for SingleCoreResolver {
	async fn resolve_core(&self, _action: &Cid) -> Result<Core, StorageError> {
		Ok(self.core.clone())
	}
}
