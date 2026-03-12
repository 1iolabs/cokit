// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_primitives::{CoDate, Date};
use std::fmt::Debug;

#[derive(Debug, Default, Clone)]
pub struct JsCoDate;
impl CoDate for JsCoDate {
	fn now(&self) -> Date {
		js_sys::Date::now() as u64
	}
}
