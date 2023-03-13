use crate::{State, Action, Reducer};

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
        Self {
            f: Box::new(f),
        }
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
