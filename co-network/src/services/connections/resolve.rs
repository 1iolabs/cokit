use async_trait::async_trait;
use co_primitives::{CoId, Network};
use std::{collections::BTreeSet, fmt::Debug, sync::Arc};

#[async_trait]
pub trait NetworkResolver: Debug {
	async fn networks(&self, co: CoId) -> Result<BTreeSet<Network>, anyhow::Error>;

	fn boxed(self) -> DynamicNetworkResolver
	where
		Self: Sized + Send + Sync + 'static,
	{
		DynamicNetworkResolver::new(self)
	}
}

#[derive(Debug, Clone)]
pub struct DynamicNetworkResolver(Arc<dyn NetworkResolver + Send + Sync + 'static>);
impl DynamicNetworkResolver {
	pub fn new(network_resolver: impl NetworkResolver + Send + Sync + 'static) -> Self {
		Self(Arc::new(network_resolver))
	}
}
#[async_trait]
impl NetworkResolver for DynamicNetworkResolver {
	async fn networks(&self, co: CoId) -> Result<BTreeSet<Network>, anyhow::Error> {
		self.0.networks(co).await
	}
}

#[cfg(test)]
#[derive(Debug, Default)]
pub struct StaticNetworkResolver(pub BTreeSet<Network>);
#[cfg(test)]
#[async_trait]
impl NetworkResolver for StaticNetworkResolver {
	async fn networks(&self, _co: CoId) -> Result<BTreeSet<Network>, anyhow::Error> {
		Ok(self.0.clone())
	}
}
