// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

#[macro_export]
macro_rules! serde_string_enum {
	($t: ident) => {
		impl std::fmt::Display for $t {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				write!(
					f,
					"{}",
					serde_json::to_value(self)
						.expect("$t to serialize")
						.as_str()
						.expect("$t to serialize to string")
				)
			}
		}
		impl TryFrom<String> for $t {
			type Error = serde_json::error::Error;
			fn try_from(value: String) -> Result<Self, Self::Error> {
				serde_json::from_value(serde_json::Value::String(value))
			}
		}
		impl TryFrom<&str> for $t {
			type Error = serde_json::error::Error;
			fn try_from(value: &str) -> Result<Self, Self::Error> {
				serde_json::from_value(serde_json::Value::String(value.to_owned()))
			}
		}
	};
}
