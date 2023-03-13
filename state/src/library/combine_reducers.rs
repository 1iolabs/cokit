use crate::types::{reducer::Reducer};

pub struct CombineReducers<R: Reducer> {
    reducers: Vec<Box<R>>,
}

impl<R: Reducer> CombineReducers<R> {
    pub fn new(reducers: Vec<Box<R>>) -> CombineReducers<R> {
        Self {
            reducers,
        }
    }
}

impl<R: Reducer> Reducer for CombineReducers<R> {
    type Action = R::Action;
    type State = R::State;
    fn reduce(&self, state: Self::State, action: &Self::Action) -> Self::State {
        self.reducers.iter().fold(state, |next_state, reducer| reducer.reduce(next_state, action))
    }
}
