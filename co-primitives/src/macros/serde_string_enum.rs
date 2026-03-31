// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

/// Adds display and to_string implementation to serde enum types.
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
