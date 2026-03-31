// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::BlockStorage;

/// Utility type to document that any block storage implementation is accepted.
pub trait AnyBlockStorage: BlockStorage + Clone + 'static {}
impl<T> AnyBlockStorage for T where T: BlockStorage + Clone + 'static {}
