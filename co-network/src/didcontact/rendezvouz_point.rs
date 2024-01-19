use super::did::ResolveResult;
use did_key::{Fingerprint, PatchedKeyPair};
use libp2p::gossipsub::IdentTopic;
use uuid::Uuid;

enum RendezvousPointSource {
	String(String),
}
pub struct RendezvousPoint {
	source: RendezvousPointSource,
}
impl RendezvousPoint {
	/// Creates a random rendevous point.
	pub fn new() -> Self {
		Self { source: RendezvousPointSource::String(Uuid::new_v4().to_string()) }
	}
}
impl From<ResolveResult> for RendezvousPoint {
	fn from(value: ResolveResult) -> Self {
		Self {
			source: match value {
				ResolveResult::Key(key) => RendezvousPointSource::String(key.fingerprint()),
			},
		}
	}
}
impl From<&PatchedKeyPair> for RendezvousPoint {
	fn from(value: &PatchedKeyPair) -> Self {
		Self { source: RendezvousPointSource::String(value.fingerprint()) }
	}
}
impl Into<Option<IdentTopic>> for &RendezvousPoint {
	/// Convert rendevous point into IdentTopic.
	fn into(self) -> Option<IdentTopic> {
		match Into::<Option<String>>::into(self) {
			Some(value) => Some(IdentTopic::new(value)),
			None => None,
		}
	}
}
impl Into<Option<String>> for &RendezvousPoint {
	/// Convert rendevous point into ASCII encoded string.
	fn into(self) -> Option<String> {
		match &self.source {
			RendezvousPointSource::String(value) => Some(value.clone()),
		}
	}
}
