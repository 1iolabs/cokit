use futures::{pin_mut, Stream, StreamExt};

#[async_trait::async_trait]
pub trait CoTryStreamExt: Stream<Item = Result<Self::Ok, Self::Error>> {
	type Ok;
	type Error;

	async fn try_first(self) -> Result<Option<Self::Ok>, Self::Error>
	where
		Self: Sized,
	{
		Ok(try_first(self).await?)
	}
}
impl<S, T, E> CoTryStreamExt for S
where
	S: ?Sized + Stream<Item = Result<T, E>>,
{
	type Ok = T;
	type Error = E;
}

async fn try_first<T, E, S>(stream: S) -> Result<Option<T>, E>
where
	S: Stream<Item = Result<T, E>> + Sized,
{
	pin_mut!(stream);
	while let Some(item) = stream.next().await {
		return match item {
			Ok(val) => Ok(Some(val)),
			Err(err) => Err(err),
		};
	}
	Ok(None)
}
