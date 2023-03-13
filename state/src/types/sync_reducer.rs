use crate::{Reducer,};
use std::fmt::Debug;
use serde::Deserialize;
use serde::Serialize;

pub trait SyncAction: Debug + Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + Unpin + 'static {}
impl<T: Debug + Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + Unpin + 'static> SyncAction for T {}

pub trait SyncState: Debug + Serialize + for<'a> Deserialize<'a> + Clone + Send + Sync + Unpin + 'static {}
impl<T: Debug + Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + Unpin + 'static> SyncState for T {}

pub trait SyncReducer: Reducer
where
    Self::State: SyncState,
    Self::Action: SyncAction,
{
    // type State: SyncState;
    // type Action: SyncAction;

    // fn reduce(&self, state: Self::State, action: &Self::Action) -> Self::State;
}

impl<T> SyncReducer for T
where
    T: Reducer + Send + 'static,
    T::Action: SyncAction,
    T::State: SyncState,
{
    // type Action = T::Action;
    // type State = T::State;

    // fn reduce(&self, state: Self::State, action: &Self::Action) -> Self::State {
    //     self.reduce(state, action)
    // }
}
