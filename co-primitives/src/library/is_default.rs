// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

/// Simple trait to check if the current value is the default.
///
/// This is particulary useful with serde:
/// ```rust
/// use serde::Serialize;
/// use co_primitives::IsDefault;
///
/// #[derive(Debug, Serialize)]
/// pub struct Hello {
///    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
///    pub world: bool,
/// }
///
/// assert_eq!(serde_json::to_string(&Hello { world: Default::default() }).unwrap(), "{}");
/// ```
pub trait IsDefault {
	fn is_default(&self) -> bool;
}
impl<T> IsDefault for T
where
	T: Default + PartialEq,
{
	fn is_default(&self) -> bool {
		&T::default() == self
	}
}
