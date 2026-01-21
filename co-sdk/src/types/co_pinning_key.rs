use anyhow::anyhow;
use co_primitives::CoId;

#[derive(Debug, Clone, PartialEq)]
pub enum CoPinningKey {
	Root,
}
impl CoPinningKey {
	pub fn to_string(&self, co: &CoId) -> String {
		match self {
			CoPinningKey::Root => format!("co:{}", co.as_str()),
		}
	}

	pub fn parse(key: String) -> Result<(CoPinningKey, CoId), anyhow::Error> {
		parse_co_id_from_pin(key)
	}
}

fn parse_co_id_from_pin(mut pin: String) -> Result<(CoPinningKey, CoId), anyhow::Error> {
	if pin.starts_with("co:") {
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
		assert_eq!(CoPinningKey::parse("co:test".to_owned()).unwrap(), (CoPinningKey::Root, CoId::from("test")));
	}
}
