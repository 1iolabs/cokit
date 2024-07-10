use anyhow::anyhow;
use co_identity::{DidCommHeader, Identity, PrivateIdentity};
use co_primitives::{serde_string_enum, Did, NetworkDidDiscovery};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DidDiscovery {
	pub network: NetworkDidDiscovery,
	pub did: Did,
	pub message_id: String,
	pub message: String,
}
impl DidDiscovery {
	/// Create DID Discovery request.
	pub fn create<F, T>(
		from: &F,
		to: &T,
		network: NetworkDidDiscovery,
		message_type: String,
	) -> Result<DidDiscovery, anyhow::Error>
	where
		F: PrivateIdentity + Send + Sync + 'static,
		T: Identity + Send + Sync + 'static,
	{
		let id: String = Uuid::new_v4().into();
		let header = DidCommHeader {
			from: Some(from.identity().to_owned()),
			to: BTreeSet::from_iter(vec![to.identity().to_owned()]),
			id: id.clone(),
			message_type,
			..Default::default()
		};
		let from_context = from
			.didcomm_private()
			.ok_or(anyhow!("unsupported identity: from: no private didcomm context"))?;
		let to_context = to
			.didcomm_public()
			.ok_or(anyhow!("unsupported identity: to: no public didcomm context"))?;
		let message = from_context.jwe(&to_context, header, "null")?;
		Ok(DidDiscovery { message_id: id, did: to.identity().to_owned(), network, message })
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DidDiscoveryMessage {
	/// Message type for a did discovery request.
	#[serde(rename = "diddiscovery")]
	Discover,

	/// Response message type to an did discovery request.
	#[serde(rename = "diddiscovery-resolve")]
	Resolve,
}
impl DidDiscoveryMessage {
	pub fn from_str(value: &str) -> Option<Self> {
		Self::try_from(value.to_owned()).ok()
	}
}
serde_string_enum!(DidDiscoveryMessage);
