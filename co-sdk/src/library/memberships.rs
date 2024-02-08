use crate::{CoReducer, CoReducerError, Cores, CO_CORE_CO, CO_CORE_MEMBERSHIP};
use co_core_co::Co;
use co_core_membership::Memberships;
use co_primitives::Tags;
use futures::Stream;
use libipld::Cid;

pub fn memberships(reducer: CoReducer) -> impl Stream<Item = Result<(String, Option<Cid>, Tags), anyhow::Error>> {
	async_stream::try_stream! {
		// root
		let co: Co = reducer.state(Cores::to_core_name(CO_CORE_CO)).await?;
		yield (co.id.clone(), reducer.reducer_state().await.0, co.tags.clone());

		// memberships
		let memberships: Memberships = match reducer.state(Cores::to_core_name(CO_CORE_MEMBERSHIP)).await {
			Ok(memberships) => memberships,
			Err(CoReducerError::CoreNotFound(_)) => Memberships::default(),
			Err(e) => Err(e)?,
		};
		for membership in memberships.memberships {
			yield (membership.id, Some(membership.co), membership.tags);
		}
	}
}
