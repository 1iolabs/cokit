use crate::EventContent;
use co_macros::co_data;
use schemars::JsonSchema;
use std::collections::BTreeMap;

/// Receipt events are used to indicate that all messages up to a specific event have been read by a user.
#[co_data]
#[derive(JsonSchema)]
pub enum ReceiptType {
	#[serde(untagged)]
	Public(PublicReceiptContent),
	#[serde(untagged)]
	Private(PrivateReceiptContent),
}

impl From<ReceiptType> for EventContent {
	fn from(val: ReceiptType) -> Self {
		EventContent::Receipt(val)
	}
}

/// These receipts are always sent into a room and indicate to all users that the messages sent up to the indicated
/// event were read by the user that sent this receipt event. This becomes public knowledge to all users
/// participating in the CO.
#[co_data]
#[derive(JsonSchema)]
pub struct PublicReceiptContent {
	/// The ID of the latest event read by the user
	#[serde(rename = "m.read")]
	pub read: String,
	/// The ID of the thread if receipt is threaded
	pub thread_id: Option<String>,
}

impl From<PublicReceiptContent> for EventContent {
	fn from(val: PublicReceiptContent) -> Self {
		ReceiptType::Public(val).into()
	}
}

/// A read receipt for one specific room. Indicates that a user has read all messages up to the given event.
#[co_data]
#[derive(JsonSchema)]
pub struct PrivateReceipt {
	/// The ID of the event the receipt references
	pub event_id: String,
	/// The ID of the thread if receipt is threaded
	pub thread_id: String,
}

/// Private read receipts are saved in a users private CO so other users cannot infer the read status. The read map
/// in this event only needs to contain the delta on the users receipts. This means that there is no need to contain
/// the complete read receipt state in this event but only the changes.
#[co_data]
#[derive(JsonSchema)]
pub struct PrivateReceiptContent {
	/// Map of all room IDs to receipts
	#[serde(rename = "m.read.private")]
	pub read: BTreeMap<String, PrivateReceipt>,
}

impl From<PrivateReceiptContent> for EventContent {
	fn from(val: PrivateReceiptContent) -> Self {
		ReceiptType::Private(val).into()
	}
}
