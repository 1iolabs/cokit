//! Utilites to work with state managed by an core.
//! This usually only involves the storage (BlockStorage) and content addresses (CID).

mod core_state;
mod dag;
mod identities;
mod memberships;
mod networks;
mod participants;
mod query;

pub use core_state::{core_state, core_state_or_default, core_state_reference};
pub use dag::{find::find, into_collection::into_collection, is_empty::is_empty, stream::stream};
pub use identities::{identities, is_identity, Identity};
pub use memberships::memberships;
pub use networks::networks;
pub use participants::{is_participant, participant_identities, participants};
pub use query::{query, query_core, Query, QueryError, QueryExt};
