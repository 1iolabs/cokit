use crate::{CoReducer, CoReducerError, Cores, CO_CORE_MEMBERSHIP, CO_CORE_NAME_CO};
use co_core_co::Co;
use co_core_membership::Memberships;
use co_primitives::{CoId, Tags};
use futures::Stream;
use libipld::Cid;

pub fn memberships(reducer: CoReducer) -> impl Stream<Item = Result<(CoId, Cid, Tags), CoReducerError>> {
	async_stream::try_stream! {
		// empty?
		let root_state = match reducer.reducer_state().await.0 {
			Some(i) => i,
			None => return
		};

		// root
		let co: Co = reducer.state(CO_CORE_NAME_CO).await?;
		yield (co.id.clone(), root_state, co.tags.clone());

		// memberships
		let memberships: Memberships = match reducer.state(Cores::to_core_name(CO_CORE_MEMBERSHIP)).await {
			Ok(memberships) => memberships,
			Err(CoReducerError::CoreNotFound(_)) => Memberships::default(),
			Err(e) => Err(e)?,
		};
		for membership in memberships.memberships {
			yield (membership.id, membership.state, membership.tags);
		}
	}
}
