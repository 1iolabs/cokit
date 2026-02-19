// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use std::fmt::Debug;

pub trait Action: Clone + Debug + Unpin + Send + Sync + 'static {}
impl<T: Clone + Debug + Unpin + Send + Sync + 'static> Action for T {}

pub trait State: Clone + Debug + Unpin + Send + Sync + 'static {}
impl<T: Clone + Debug + Unpin + Send + Sync + 'static> State for T {}

/// State reducer.
///
/// Reduces previous state and action to next state.
///
/// Possible errors should be handled by adding them into the state.
/// Unhandled errors will panic.
pub trait Reducer {
	type State: State;
	type Action: Action;
	fn reduce(&self, state: Self::State, action: &Self::Action) -> Self::State;
}
