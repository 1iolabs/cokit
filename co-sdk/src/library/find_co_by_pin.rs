use crate::{CoContext, CoPinningKey, CoReducer, CoReducerFactory};

pub async fn find_co_by_pin(context: &CoContext, pin: String) -> Result<CoReducer, anyhow::Error> {
	let (_pinning_key, co_id) = CoPinningKey::parse(pin)?;
	let co = context.try_co_reducer(&co_id).await?;
	Ok(co)
}
