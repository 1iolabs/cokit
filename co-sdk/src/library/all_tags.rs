use crate::{CoContext, CoReducerFactory};
use anyhow::anyhow;
use co_primitives::{CoId, Tag};
use futures::Stream;

/// Get all tags starting at `id` going up parents.
pub fn all_tags<'a, 'b: 'a>(context: &'a CoContext, id: &'b CoId) -> impl Stream<Item = anyhow::Result<Tag>> + 'a {
	async_stream::try_stream! {
		// co
		let (mut parent, mut items) = tags(context, id).await?;
		for tag in items {
			yield tag;
		}

		// parents
		while let Some(id) = parent {
			(parent, items) = tags(context, &id).await?;
			for tag in items {
				yield tag;
			}
		}
	}
}

async fn tags(context: &CoContext, id: &CoId) -> anyhow::Result<(Option<CoId>, impl Iterator<Item = Tag>)> {
	let co = context.co_reducer(id).await?.ok_or(anyhow!("Unknown CO: {id}"))?;
	let (_storage, co_state) = co.co().await?;
	Ok((co.parent_id().cloned(), co_state.tags.into_iter()))
}
