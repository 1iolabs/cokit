// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

// modules
mod runtime;
#[cfg(wasmer_backend)]
pub mod wasmer;

// export
#[cfg(wasmer_backend)]
pub use self::wasmer::create_runtime;
pub use runtime::{Runtime, RuntimeBox, RuntimeError};
