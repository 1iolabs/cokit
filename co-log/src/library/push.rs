use crate::{Entry, Log};
use co_storage::Storage;
use libipld::Cid;

pub fn push(log: &mut Log, storage: &mut dyn Storage, data: Cid) -> Entry {
	todo!()
}
