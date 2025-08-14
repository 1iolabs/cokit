use crate::{
	state::{query_core, Query, QueryError},
	CO_CORE_NAME_CO,
};
use co_core_co::Co;
use co_primitives::{OptionLink, Tags};
use co_storage::BlockStorage;

pub async fn co<S>(storage: &S, co_state: OptionLink<Co>) -> Result<Co, QueryError>
where
	S: BlockStorage + Clone + 'static,
{
	Ok(query_core(CO_CORE_NAME_CO).with_default().execute(storage, co_state).await?)
}

pub async fn co_info<S>(storage: &S, co_state: OptionLink<Co>) -> Result<CoInfo, QueryError>
where
	S: BlockStorage + Clone + 'static,
{
	let co = query_core(CO_CORE_NAME_CO).with_default().execute(storage, co_state).await?;
	Ok(CoInfo { name: co.name, tags: co.tags })
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CoInfo {
	pub name: String,
	pub tags: Tags,
}
