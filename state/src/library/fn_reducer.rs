// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{Action, Reducer, State};

pub struct FnReducer<S, A>
where
	S: Action,
	A: State,
{
	f: Box<dyn Fn(S, &A) -> S + Send + 'static>,
}
impl<S, A> FnReducer<S, A>
where
	S: Action,
	A: State,
{
	pub fn new(f: impl Fn(S, &A) -> S + Send + 'static) -> Self {
		Self { f: Box::new(f) }
	}
}
impl<S, A> Reducer for FnReducer<S, A>
where
	S: Action,
	A: State,
{
	type State = S;
	type Action = A;

	fn reduce(&self, state: Self::State, action: &Self::Action) -> Self::State {
		(self.f)(state, action)
	}
}
