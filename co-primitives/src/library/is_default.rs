/// Simple trait to check if the current value is the default.
///
/// This is particulary useful with serde:
/// ```rust
/// pub struct Hello {
///    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
///    pub world: bool;
/// }
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
