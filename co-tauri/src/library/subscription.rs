use co_sdk::CoId;

pub fn build_event_name(co: CoId, core: Option<&str>) -> String {
	match core {
		Some(core) => format!("{co}/{core}"),
		None => co.to_string(),
	}
}
