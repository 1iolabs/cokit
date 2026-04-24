// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

#[cfg(feature = "llvm")]
pub mod compile;
#[cfg(feature = "js")]
pub mod deferred_storage;
pub mod instance;
#[cfg(wasmer_backend)]
pub mod module_description;
pub mod pool;
