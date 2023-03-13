use crate::{Reducer, StoreApi};


/// Store middleware which allows to modify dispatch behaviour.
/// The difference to an Reducer is basically the &mut self reference because reducers are required to be pure.
pub trait Middleware<R: Reducer> {
    fn dispatch<'a>(&mut self, next: &'a mut dyn StoreApi<R>, action: R::Action);
}
