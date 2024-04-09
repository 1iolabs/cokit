use futures::{Stream, StreamExt};
use libipld::Cid;
use rxrust::{observable::ObservableExt, subject::SubjectThreads};
use std::{collections::BTreeSet, convert::Infallible};

pub struct StateObservable {
	pub(crate) sub: SubjectThreads<(Cid, BTreeSet<Cid>), Infallible>,
}
impl StateObservable {
	pub fn stream(&self) -> impl Stream<Item = (Cid, BTreeSet<Cid>)> {
		self.sub
			.clone()
			.to_stream()
			.map(|result| result.expect("Infallible to not fail"))
	}
}
