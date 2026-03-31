// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
