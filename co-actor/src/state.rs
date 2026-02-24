// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

/// State reducer.
///
/// # Abstraction Goals
/// - Separation of concerns: This is only intended to apply actions ot state.
/// - Make state changes trivialy testable.
/// - Deterministic state changes.
/// - Deterministic observe changes of interest (retuned actions).
pub trait Reducer<A>
where
	A: Clone + Send + 'static,
	Self: Sized,
{
	fn reduce(&mut self, action: A) -> Vec<A>;
}
