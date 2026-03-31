// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use co_primitives::{CoDate, Date};
use std::fmt::Debug;

#[derive(Debug, Default, Clone)]
pub struct JsCoDate;
impl CoDate for JsCoDate {
	fn now(&self) -> Date {
		js_sys::Date::now() as u64
	}
}
