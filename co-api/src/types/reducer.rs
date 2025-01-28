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
}

pub mod async_reducer {
	use cid::Cid;
	use co_primitives::{BlockStorage, Link, OptionLink, ReducerAction};

	/// COre execution context.
	pub trait Context<S> {
		/// Storage instance.
		fn storage(&self) -> &S;

		/// Get action to apply to the state.
		fn event(&self) -> Cid;

		/// Get current COre state.
		/// Returns [`None`] if no prior state.
		fn state(&self) -> Option<Cid>;

		/// Set next COre state.
		fn set_state(&mut self, cid: Cid);

		/// Signal error.
		fn set_error(&mut self, error: anyhow::Error);
	}

	#[allow(async_fn_in_trait)]
	pub trait Reducer<A, S>
	where
		Self: Sized,
		A: Clone,
		S: BlockStorage,
	{
		async fn reduce(
			state: OptionLink<Self>,
			event: ReducerAction<A>,
			storage: &S,
		) -> Result<Link<Self>, anyhow::Error>;
	}

	// #[async_trait]
	// pub trait Storage {
	// 	/// Returns a block from storage.
	// 	async fn get(&self, cid: &Cid) -> Result<Block<DefaultParams>, StorageError>;

	// 	/// Inserts a block into storage.
	// 	async fn set(&self, block: Block<DefaultParams>) -> Result<Cid, StorageError>;
	// }

	// #[async_trait]
	// pub trait StorageReferences {
	// 	/// References a block.
	// 	/// Returns all [`Cid`] which has been references by this call.
	// 	async fn alloc(&self, alloc: Cid, recursive: bool) -> BTreeSet<Cid>;

	// 	/// Unreference block.
	// 	/// Returns all [`Cid`] which has been unreferences by this call.
	// 	async fn free(&self, free: Cid, recursive: bool) -> BTreeSet<Cid>;

	// 	/// Unreference block and reference another.
	// 	async fn replace(&self, from: Cid, to: Cid) -> BTreeSet<Cid>;
	// }
}
