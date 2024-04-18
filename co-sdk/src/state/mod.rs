//! Utilites to work with state managed by an core.
//! This usually only involves the storage (BlockStorage) and content addresses (CID).

mod core_state;
mod dag;
mod identities;
mod memberships;

pub use core_state::{core_state, core_state_or_default};
pub use dag::{find::find, into_collection::into_collection, stream::stream};
pub use identities::{identities, Identity};
pub use memberships::memberships;
