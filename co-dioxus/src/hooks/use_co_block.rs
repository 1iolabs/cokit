use crate::{use_co_context, use_co_error, CoContext, CoError, CoErrorSignal};
use anyhow::anyhow;
use co_sdk::{BlockStorage, BlockStorageExt, CoId};
use dioxus::prelude::*;
use libipld::{Block, Cid, DefaultParams};
use serde::de::DeserializeOwned;

pub fn use_co_block(co: impl Into<CoId>, cid: Cid) -> Signal<Option<Block<DefaultParams>>, SyncStorage> {
	let signal = use_signal_sync(|| None);
	let error = use_co_error();
	let context = use_co_context();
	let mut hook = use_hook(|| CoBlockHook { arguments: None });
	hook.fetch(context, signal, error, co, cid);
	signal
}

pub fn use_co_block_deserialized<T: DeserializeOwned + Send + Sync>(
	co: impl Into<CoId>,
	cid: Cid,
) -> Signal<Option<T>, SyncStorage> {
	let signal = use_signal_sync(|| None);
	let error = use_co_error();
	let context = use_co_context();
	let mut hook = use_hook(|| CoBlockHook { arguments: None });
	hook.fetch_deserialized(context, signal, error, co, cid);
	signal
}

#[derive(Clone)]
struct CoBlockHook {
	arguments: Option<Cid>,
}
impl CoBlockHook {
	fn fetch(
		&mut self,
		context: CoContext,
		mut signal: Signal<Option<Block<DefaultParams>>, SyncStorage>,
		mut error: CoErrorSignal,
		co: impl Into<CoId>,
		cid: Cid,
	) {
		if self.arguments == Some(cid) {
			return;
		}
		self.arguments = Some(cid);
		let co = co.into();
		context.execute_future_parallel(move |application| async move {
			let co_reducer = match application.co_reducer(&co).await {
				Ok(Some(i)) => i,
				Ok(None) => {
					error.write().push(CoError::from_error(anyhow!("Co not found: {}", co)));
					return;
				},
				Err(err) => {
					error.write().push(CoError::from_error(err));
					return;
				},
			};
			match co_reducer.storage().get(&cid).await {
				Ok(block) => {
					*signal.write() = Some(block);
				},
				Err(err) => {
					*signal.write() = None;
					error.write().push(CoError::from_error(err));
				},
			}
		});
	}

	fn fetch_deserialized<T: DeserializeOwned + Send + Sync>(
		&mut self,
		context: CoContext,
		mut signal: Signal<Option<T>, SyncStorage>,
		mut error: CoErrorSignal,
		co: impl Into<CoId>,
		cid: Cid,
	) {
		if self.arguments == Some(cid) {
			return;
		}
		self.arguments = Some(cid);
		let co = co.into();
		context.execute_future_parallel(move |application| async move {
			let co_reducer = match application.co_reducer(&co).await {
				Ok(Some(i)) => i,
				Ok(None) => {
					error.write().push(CoError::from_error(anyhow!("Co not found: {}", co)));
					return;
				},
				Err(err) => {
					error.write().push(CoError::from_error(err));
					return;
				},
			};
			match co_reducer.storage().get_deserialized::<T>(&cid).await {
				Ok(block) => {
					*signal.write() = Some(block);
				},
				Err(err) => {
					*signal.write() = None;
					error.write().push(CoError::from_error(err));
				},
			}
		});
	}
}
