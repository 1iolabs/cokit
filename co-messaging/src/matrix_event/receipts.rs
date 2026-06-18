// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::EventContent;
use co_macros::co;
use schemars::JsonSchema;
use std::collections::BTreeMap;

/// These receipts are always sent into a room and indicate to all users that the messages sent up to the indicated
/// event were read by the user that sent this receipt event. This becomes public knowledge to all users
/// participating in the CO.
#[co]
#[derive(JsonSchema, Default)]
#[non_exhaustive]
pub struct PublicReceiptContent {
	/// The ID of the latest event read by the user
	#[serde(rename = "m_read")]
	pub read: String,
	/// The ID of the thread if receipt is threaded
	pub thread_id: Option<String>,
}

impl PublicReceiptContent {
	pub fn new(read: impl Into<String>) -> Self {
		Self { read: read.into(), ..Default::default() }
	}
}

impl From<PublicReceiptContent> for EventContent {
	fn from(val: PublicReceiptContent) -> Self {
		EventContent::Receipt(val)
	}
}

// TODO move to another core as these should not be visible to other co participants
/// A read receipt for one specific room. Indicates that a user has read all messages up to the given event.
#[co]
#[derive(JsonSchema, Default)]
#[non_exhaustive]
pub struct PrivateReceipt {
	/// The ID of the event the receipt references
	pub event_id: String,
	/// The ID of the thread if receipt is threaded
	pub thread_id: String,
}

impl PrivateReceipt {
	pub fn new(event_id: impl Into<String>, thread_id: impl Into<String>) -> Self {
		Self { event_id: event_id.into(), thread_id: thread_id.into() }
	}
}

/// Private read receipts are saved in a users private CO so other users cannot infer the read status. The read map
/// in this event only needs to contain the delta on the users receipts. This means that there is no need to contain
/// the complete read receipt state in this event but only the changes.
#[co]
#[derive(JsonSchema, Default)]
#[non_exhaustive]
pub struct PrivateReceiptContent {
	/// Map of all room IDs to receipts
	#[serde(rename = "m.read.private")]
	pub read: BTreeMap<String, PrivateReceipt>,
}

impl PrivateReceiptContent {
	pub fn new(read: BTreeMap<String, PrivateReceipt>) -> Self {
		Self { read }
	}
}
