// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::CoContext;
use dioxus::prelude::*;

pub fn use_co_context() -> CoContext {
	use_context()
}
