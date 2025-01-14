use co_sdk::CoId;

/// Builds the ID for a core in a co. Can be used to then identify a unique core in a specified co.
/// Can be used to specify a subscription target
pub fn build_core_id(co: CoId, core: Option<&str>) -> String {
	match core {
		Some(core) => format!("{co}/{core}"),
		None => co.to_string(),
	}
}
