// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{types::cores::CO_CORE_NAME_PIN, CoReducer};
use anyhow::Ok;
use cid::Cid;
use co_core_pin::PinAction;
use co_identity::PrivateIdentity;
use co_primitives::Tags;
use co_storage::BlockStorage;
use std::fmt::Debug;

#[derive(Clone)]
pub struct PinAPI<'a, I> {
	pub co_reducer: &'a CoReducer,
	pub identity: &'a I,
}

impl<'a, I> PinAPI<'a, I>
where
	I: PrivateIdentity + Debug + Clone + Send + Sync + 'static,
{
	pub fn api(co_reducer: &'a CoReducer, identity: &'a I) -> Self {
		Self { co_reducer, identity }
	}
	pub async fn pin_cid(&self, cid: Cid, tags: Tags) -> Result<(), anyhow::Error> {
		self.co_reducer
			.push(self.identity, CO_CORE_NAME_PIN, &PinAction::Pin(cid, tags))
			.await?;
		Ok(())
	}
	pub async fn unpin_cid(&self, cid: Cid, tags: Tags) -> Result<(), anyhow::Error> {
		self.co_reducer
			.push(self.identity, CO_CORE_NAME_PIN, &PinAction::Unpin(cid, tags))
			.await?;
		Ok(())
	}
	pub async fn unpin_all(&self, tags: Tags) -> Result<(), anyhow::Error> {
		self.co_reducer
			.push(self.identity, CO_CORE_NAME_PIN, &PinAction::UnpinAll(tags))
			.await?;
		Ok(())
	}
	pub async fn clean_storage<S>(&self, _storage: S) -> Result<(), anyhow::Error>
	where
		S: Iterator<Item = Cid> + BlockStorage + Clone,
	{
		// TODO
		// let pins = self.co_reducer.state::<Pin>(CO_CORE_NAME_PIN).await?.pins;
		// let pinned_cids = NodeStream::from_node_container(self.co_reducer.storage(), &pins)
		// 	.map_ok(|v| v.0)
		// 	.try_collect::<BTreeSet<Cid>>()
		// 	.await?;
		// for cid in storage.clone() {
		// 	storage.remove(&cid);
		// }
		Ok(())
	}
}
