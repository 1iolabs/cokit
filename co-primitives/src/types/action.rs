// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{Date, Did};
use ipld_core::{
	ipld::Ipld,
	serde::{from_ipld, to_ipld},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReducerAction<T> {
	/// Sender.
	#[serde(rename = "f")]
	pub from: Did,

	/// Time when the event occured.
	///
	/// Note: The time from the dispatching device is used.
	#[serde(rename = "t")]
	pub time: Date,

	/// COre affected by this action.
	#[serde(rename = "c")]
	pub core: String,

	/// Action payload.
	#[serde(rename = "p")]
	pub payload: T,
}
impl ReducerAction<Ipld> {
	pub fn set_payload<T: Serialize>(&mut self, value: &T) -> Result<(), String> {
		self.payload = to_ipld(value).map_err(|e| e.to_string())?;
		Ok(())
	}

	pub fn get_payload<T: DeserializeOwned>(&self) -> Result<T, String> {
		from_ipld(self.payload.clone()).map_err(|e| e.to_string())
	}
}
