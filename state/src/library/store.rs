use crate::{Middleware, Reducer, StoreApi};
use std::fmt::Debug;

/// Simple local state storage implementation.
pub struct Store<R>
where
    R: Reducer + Send + 'static,
{
    state: Option<R::State>,
    state_default: R::State,
    reducer: Box<R>,
    on_changed: Box<dyn Fn(&R::State) + Send + 'static>,
}

impl<R> Debug for Store<R>
where
    R: Reducer + Send + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalStore")
            .field("state", &self.state)
            .field("state_default", &self.state_default)
            .finish()
    }
}

impl<R> Store<R>
where
    R: Reducer + Send + 'static,
{
    pub fn new(
        state: R::State,
        on_changed: impl Fn(&R::State) + Send + 'static,
        reducer: R,
    ) -> Self {
        Self {
            state: Some(state.clone()),
            state_default: state.clone(),
            reducer: Box::new(reducer),
            on_changed: Box::new(on_changed),
        }
    }

    pub fn set_reducer(&mut self, reducer: R) {
        self.reducer = Box::new(reducer);
    }

    pub fn set_on_changed(&mut self, on_changed: impl Fn(&R::State) + Send + 'static) {
        self.on_changed = Box::new(on_changed);
    }

    pub fn select<F: FnOnce(&R::State) -> T, T>(&self, f: F) -> T {
        (f)(&self.state.as_ref().unwrap_or(&self.state_default))
    }
}

impl<R> StoreApi<R> for Store<R>
where
    R: Reducer + Send + 'static,
{
    fn dispatch(&mut self, action: R::Action) {
        self.state = Some(
            self.reducer.reduce(
                self.state
                    .take()
                    .unwrap_or_else(|| self.state_default.clone()),
                &action,
            ),
        );
        (self.on_changed)(self.state.as_ref().unwrap());
    }

    fn state(&self) -> R::State {
        // self.state.as_ref().unwrap_or(&self.state_default).clone()
        self.state
            .clone()
            .expect("Calling state() while dispatching is not allowed")
    }

    fn with_middleware(
        self: Box<Self>,
        middleware: Box<dyn Middleware<R> + Send + 'static>,
    ) -> Box<dyn StoreApi<R> + Send + 'static> {
        Box::new(MiddlewareStore {
            middleware,
            next: self,
        })
    }
}

pub struct MiddlewareStore<R>
where
    R: Reducer + Send + 'static,
{
    next: Box<dyn StoreApi<R> + Send + 'static>,
    middleware: Box<dyn Middleware<R> + Send + 'static>,
}

impl<R> MiddlewareStore<R>
where
    R: Reducer + Send + 'static,
{
    pub fn new(
        next: Box<dyn StoreApi<R> + Send + 'static>,
        middleware: Box<dyn Middleware<R> + Send + 'static>,
    ) -> Self {
        Self { next, middleware }
    }
}

impl<R> Debug for MiddlewareStore<R>
where
    R: Reducer + Send + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MiddlewareStore").finish()
    }
}

impl<R> StoreApi<R> for MiddlewareStore<R>
where
    R: Reducer + Send + 'static,
{
    fn dispatch(&mut self, action: R::Action) {
        self.middleware
            .as_mut()
            .dispatch(self.next.as_mut(), action);
    }

    fn state(&self) -> R::State {
        self.next.state()
    }

    fn with_middleware(
        self: Box<Self>,
        middleware: Box<dyn Middleware<R> + Send + 'static>,
    ) -> Box<dyn StoreApi<R> + Send + 'static> {
        Box::new(MiddlewareStore {
            middleware,
            next: self,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::{self, channel, Sender};
    use crate::{FnReducer, Middleware, Reducer, Store, StoreApi};

    #[test]
    fn dispatch() {
        #[derive(Clone, Debug)]
        enum IntAction {
            Inc,
            Dec,
        }
        let reducer = FnReducer::new(|state, action: &_| match action {
            IntAction::Inc => state + 1,
            IntAction::Dec => state - 1,
        });
        let (tx, rx) = mpsc::sync_channel(1);
        let on_changed = move |state: &_| {
            tx.send(*state).unwrap();
        };
        let mut store = Store::new(0, on_changed, reducer);
        store.dispatch(IntAction::Inc);
        assert_eq!(rx.recv().unwrap(), 1);
        store.dispatch(IntAction::Dec);
        assert_eq!(rx.recv().unwrap(), 0);
    }

    #[test]
    fn middleware() {
        struct TestMiddleware<R: Reducer> {
            states: Sender<(R::Action, R::State, R::State)>,
        }
        impl<R: Reducer> Middleware<R> for TestMiddleware<R>
        where
            R::Action: Clone,
            R::State: Clone,
        {
            fn dispatch<'a>(&mut self, next: &'a mut dyn StoreApi<R>, action: R::Action) {
                let previous_state = next.state();
                next.dispatch(action.clone());
                let next_state = next.state();
                self.states
                    .send((action, previous_state, next_state))
                    .unwrap();
            }
        }

        #[derive(Clone, Debug, PartialEq)]
        enum IntAction {
            Inc,
            Dec,
        }
        let reducer = FnReducer::new(|state, action: &_| match action {
            IntAction::Inc => state + 1,
            IntAction::Dec => state - 1,
        });
        let on_changed = |_: &_| {};
        let store_1 = Box::new(Store::new(0, on_changed, reducer));
        let (tx, rx) = channel();
        let middleware = Box::new(TestMiddleware { states: tx });
        let mut store = store_1.with_middleware(middleware);
        store.dispatch(IntAction::Inc);
        store.dispatch(IntAction::Dec);
        assert_eq!(rx.recv().unwrap(), (IntAction::Inc, 0, 1));
        assert_eq!(rx.recv().unwrap(), (IntAction::Dec, 1, 0));
    }
}
