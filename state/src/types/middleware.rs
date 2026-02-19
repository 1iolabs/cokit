// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{Reducer, StoreApi};

/// Store middleware which allows to modify dispatch behaviour.
/// The difference to an Reducer is basically the &mut self reference because reducers are required to be pure.
pub trait Middleware<R: Reducer> {
	fn dispatch<'a>(&mut self, next: &'a mut dyn StoreApi<R>, action: R::Action);
}
