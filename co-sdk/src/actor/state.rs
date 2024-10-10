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
