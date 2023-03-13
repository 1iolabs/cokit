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
