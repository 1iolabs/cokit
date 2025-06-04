use crate::{Cid, ReducerAction, Storage};

pub trait Reducer {
	type Action: Clone;

	fn reduce(self, event: &ReducerAction<Self::Action>, context: &mut dyn Context) -> Self;
}

pub trait Context {
	fn storage(&self) -> &dyn Storage;

	fn storage_mut(&mut self) -> &mut dyn Storage;

	fn event(&self) -> Cid;

	fn state(&self) -> Option<Cid>;

	fn store_state(&mut self, cid: Cid);

	fn write_diagnostic(&mut self, cid: Cid);
}

pub mod async_reducer {
	use cid::Cid;
	use co_primitives::{BlockStorage, Link, OptionLink, ReducerAction};

	/// COre execution context.
	pub trait Context<S>
	where
		S: BlockStorage + Clone + 'static,
	{
		/// Storage instance.
		fn storage(&self) -> &S;

		/// Get action to apply to the state.
		fn event(&self) -> Cid;

		/// Get current COre state.
		/// Returns [`None`] if no prior state.
		fn state(&self) -> Option<Cid>;

		/// Set next COre state.
		fn set_state(&mut self, cid: Cid);

		/// Write diagnostic block.
		fn write_diagnostic(&mut self, cid: Cid);
	}

	#[allow(async_fn_in_trait)]
	pub trait Reducer<A, S>
	where
		Self: Sized,
		A: Clone,
		S: BlockStorage + Clone + 'static,
	{
		async fn reduce(
			state: OptionLink<Self>,
			event: Link<ReducerAction<A>>,
			storage: &S,
		) -> Result<Link<Self>, anyhow::Error>;
	}
}
