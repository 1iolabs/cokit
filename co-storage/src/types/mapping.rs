use async_trait::async_trait;
use cid::Cid;

pub trait StorageContentMapping {
	/// Convert the mapped [`Cid`] to an plain storage [`Cid`].
	fn to_plain(&self, mapped: &Cid) -> Option<Cid>;

	/// Convert the plain storage [`Cid`] to a mapped [`Cid`].
	fn to_mapped(&self, plain: &Cid) -> Option<Cid>;
}

/// Map [`Cid`]s between mappend and plain.
///
/// Plain:
/// - External.
/// - The Cid stored on disk.
/// - For example a Encrypted Block.
///
/// Mapped:
/// - Internal.
/// - The Cid used for references.
/// - Unencrypted.
#[async_trait]
pub trait BlockStorageContentMapping: Send + Sync {
	/// Whether the mapping is active.
	async fn is_content_mapped(&self) -> bool {
		false
	}

	/// Convert the mapped [`Cid`] to an plain storage [`Cid`].
	async fn to_plain(&self, mapped: &Cid) -> Option<Cid> {
		let _mapped = mapped; // prevent warning
		None
	}

	/// Convert the plain storage [`Cid`] to a mapped [`Cid`].
	async fn to_mapped(&self, plain: &Cid) -> Option<Cid> {
		let _plain = plain; // prevent warning
		None
	}
}
