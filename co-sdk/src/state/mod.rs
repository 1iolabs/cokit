// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

pub mod board;
mod co;
mod dag;
mod identities;
mod memberships;
mod networks;
mod participants;
mod query;

pub use co::{
	core::{core, core_or_default, core_state},
	info::{co, co_info},
};
pub use dag::{find::find, into_collection::into_collection, is_empty::is_empty, stream::stream};
pub use identities::{identities, is_identity, Identity};
pub use memberships::memberships;
pub use networks::networks;
pub use participants::{is_participant, participant_identities, participants, participants_active};
pub use query::{query, query_core, Query, QueryError, QueryExt};
