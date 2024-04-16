//! Utilites to work with state managed by an core.
//! This usually only involves the storage (BlockStorage) and content addresses (CID).

mod core_state;
mod dag;
mod memberships;

pub use core_state::core_state;
pub use dag::{find::find, into_collection::into_collection, stream::stream};
pub use memberships::memberships;
