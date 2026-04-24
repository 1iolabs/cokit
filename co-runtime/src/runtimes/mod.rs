// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

// modules
mod runtime;
#[cfg(wasmer_backend)]
pub mod wasmer;

// export
pub use runtime::{Runtime, RuntimeBox, RuntimeError};
#[cfg(wasmer_backend)]
pub use self::wasmer::create_runtime;
