// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

pub mod async_reducer {
	use crate::CoreBlockStorage;
	use cid::Cid;
	use co_primitives::{Link, OptionLink, ReducerAction};

	/// COre execution context.
	pub trait Context {
		/// Storage instance.
		fn storage(&self) -> &CoreBlockStorage;

		/// Get runtime payload.
		fn payload(&self) -> Vec<u8>;

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
	pub trait Reducer<A>
	where
		Self: Sized,
		A: Clone,
	{
		async fn reduce(
			state: OptionLink<Self>,
			event: Link<ReducerAction<A>>,
			storage: &CoreBlockStorage,
		) -> Result<Link<Self>, anyhow::Error>;
	}
}
