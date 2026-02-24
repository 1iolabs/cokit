// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

mod api;
mod dynamic;
mod join;
#[cfg(feature = "pinning")]
mod local_storage;
mod membership;
mod static_state_resolver;
#[cfg(feature = "pinning")]
mod storage;

pub use api::{StateResolver, StateResolverContext, StateStream};
pub use dynamic::DynamicStateResolver;
pub use join::JoinStateResolver;
#[cfg(feature = "pinning")]
pub use local_storage::LocalStorageStateResolver;
pub use membership::MembershipStateResolver;
pub use static_state_resolver::StaticStateResolver;
#[cfg(feature = "pinning")]
pub use storage::StorageStateResolver;
