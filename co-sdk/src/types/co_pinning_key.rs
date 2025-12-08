use anyhow::anyhow;
use co_primitives::CoId;

#[derive(Debug, Clone, PartialEq)]
pub enum CoPinningKey {
	State,
	Log,
	Root,
}
impl CoPinningKey {
	pub fn to_string(&self, co: &CoId) -> String {
		match self {
			CoPinningKey::State => format!("co.state.{}", co.as_str()),
			CoPinningKey::Log => format!("co.log.{}", co.as_str()),
			CoPinningKey::Root => format!("co:{}", co.as_str()),
		}
	}

	pub fn parse(key: String) -> Result<(CoPinningKey, CoId), anyhow::Error> {
		parse_co_id_from_pin(key)
	}
}

fn parse_co_id_from_pin(mut pin: String) -> Result<(CoPinningKey, CoId), anyhow::Error> {
	if pin.starts_with("co.state.") {
		Ok((CoPinningKey::State, pin.split_off("co.state.".len()).into()))
	} else if pin.starts_with("co.log.") {
		Ok((CoPinningKey::Log, pin.split_off("co.log.".len()).into()))
	} else if pin.starts_with("co:") {
		Ok((CoPinningKey::Root, pin.split_off("co:".len()).into()))
	} else {
		Err(anyhow!("Parse pin failed: {}", pin))
	}
}

#[cfg(test)]
mod tests {
	use super::CoPinningKey;
	use co_primitives::CoId;

	#[test]
	fn test_parse() {
		assert_eq!(CoPinningKey::parse("co.log.test".to_owned()).unwrap(), (CoPinningKey::Log, CoId::from("test")));
		assert_eq!(CoPinningKey::parse("co.state.test".to_owned()).unwrap(), (CoPinningKey::State, CoId::from("test")));
	}
}
