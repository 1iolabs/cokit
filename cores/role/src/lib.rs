// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_api::{co, BlockStorageExt, CoMap, CoSet, CoreBlockStorage, Did, Link, OptionLink, Reducer, ReducerAction};

#[co(state)]
pub struct Roles {
	pub roles: CoMap<Did, CoSet<Role>>,
}

#[co]
pub struct Role {}

#[co]
pub enum RoleAction {}

impl Reducer<RoleAction> for Roles {
	async fn reduce(
		state: OptionLink<Self>,
		event: Link<ReducerAction<RoleAction>>,
		storage: &CoreBlockStorage,
	) -> Result<Link<Self>, anyhow::Error> {
		let _action = storage.get_value(&event).await?;
		let result = storage.get_value_or_default(&state).await?;
		Ok(storage.set_value(&result).await?)
	}
}
