// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use co_sdk::CoId;
use dioxus::{
	hooks::{use_memo, use_reactive},
	signals::ReadSignal,
};

/// Use co id reactive.
pub fn use_co_id(co: String) -> ReadSignal<CoId> {
	use_memo(use_reactive(&co, CoId::new)).into()
}
