use crate::{state::core_state, CoReducerError, CoStorage, CO_CORE_NAME_CO, CO_CORE_NAME_MEMBERSHIP};
use co_core_co::Co;
use co_core_membership::{MembershipState, Memberships};
use co_primitives::{CoId, OptionLink, Tags};
use futures::Stream;
use libipld::Cid;

/// Returns memberships contained in the CO (`co_state`)`.
///
/// # Warning
/// - This will return all memberships and will not filter by [`co_core_membership::Membership::membership_state`].
///
/// # Arguments
/// - `storage` - The BlockStorage.
/// - `co_state` - Co Core State (`co-core-co`).
pub fn memberships(
	storage: CoStorage,
	co_state: OptionLink<co_core_co::Co>,
) -> impl Stream<Item = Result<(CoId, Cid, Tags, MembershipState), CoReducerError>> {
	async_stream::try_stream! {
		// root
		let co: Co = core_state(&storage, co_state, CO_CORE_NAME_CO).await?.1;
		if let Some(co_state) = co_state.cid() {
			yield (co.id.clone(), *co_state, co.tags.clone(), MembershipState::Active);
		}

		// memberships
		let memberships: Memberships = match core_state(&storage, co_state, CO_CORE_NAME_MEMBERSHIP).await {
			Ok((_, memberships)) => memberships,
			Err(CoReducerError::CoreNotFound(_)) => Memberships::default(),
			Err(e) => Err(e)?,
		};
		for membership in memberships.memberships {
			yield (membership.id, membership.state, membership.tags, membership.membership_state);
		}
	}
}
