use async_trait::async_trait;
use libipld::Cid;

pub trait StorageContentMapping {
	/// Convert the mapped [`Cid`] to an plain storage [`Cid`].
	fn to_plain(&self, mapped: &Cid) -> Option<Cid>;

	/// Convert the plain storage [`Cid`] to a mapped [`Cid`].
	fn to_mapped(&self, plain: &Cid) -> Option<Cid>;
}

#[async_trait]
pub trait BlockStorageContentMapping {
	/// Convert the mapped [`Cid`] to an plain storage [`Cid`].
	async fn to_plain(&self, mapped: &Cid) -> Option<Cid>;

	/// Convert the plain storage [`Cid`] to a mapped [`Cid`].
	async fn to_mapped(&self, plain: &Cid) -> Option<Cid>;
}
