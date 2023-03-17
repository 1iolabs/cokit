use crate::{Middleware, Reducer};

pub struct LogMiddleware {}

impl LogMiddleware {
    pub fn new() -> Self {
        Self {}
    }
}

impl<R> Middleware<R> for LogMiddleware
where
    R: Reducer + 'static,
{
    fn dispatch<'a>(
        &mut self,
        next: &'a mut dyn crate::StoreApi<R>,
        action: <R as Reducer>::Action,
    ) {
        println!("dispatch: {:?}", action);

        // next
        next.dispatch(action);
    }
}
