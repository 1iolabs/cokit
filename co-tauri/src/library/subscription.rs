use co_sdk::CoId;
use futures::lock::Mutex;
use std::collections::{BTreeMap, BTreeSet};

pub struct Subscriptions {
	pub active_subscriptions: Mutex<BTreeMap<String, BTreeSet<String>>>,
}

pub fn build_event_name(co: CoId, core: Option<&str>) -> String {
	match core {
		Some(core) => format!("{co}/{core}"),
		None => co.to_string(),
	}
}
