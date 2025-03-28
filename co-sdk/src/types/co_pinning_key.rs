use co_primitives::CoId;

pub enum CoPinningKey {
	State,
	Log,
}
impl CoPinningKey {
	pub fn to_string(&self, co: &CoId) -> String {
		match self {
			CoPinningKey::State => format!("co.{}.state", co.as_str()),
			CoPinningKey::Log => format!("co.{}.log", co.as_str()),
		}
	}
}
