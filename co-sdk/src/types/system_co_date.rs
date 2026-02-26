// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_primitives::{CoDate, Date};
use std::{
	fmt::Debug,
	time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Default, Clone)]
pub struct SystemCoDate;
impl CoDate for SystemCoDate {
	fn now(&self) -> Date {
		SystemTime::now().duration_since(UNIX_EPOCH).expect("Valid time").as_millis() as u64
	}
}
