// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

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
