use crate::{Action, CoContext, Epic, Observable};
use co_core_co::Co;
use co_primitives::{CoId, Link};
use futures::{pin_mut, StreamExt};

pub type ActionObservable = Observable<Action>;
pub type State = (CoId, Link<Co>);
pub type StateObservable = Observable<State>;
pub type Context = CoContext;

#[derive(Debug, Default, Clone)]
pub struct ReactiveContext {
	actions: ActionObservable,
	states: StateObservable,
}
impl ReactiveContext {
	pub fn actions(&self) -> &ActionObservable {
		&self.actions
	}

	pub fn states(&self) -> &StateObservable {
		&self.states
	}

	pub fn shutdown(&self) {
		self.actions.shutdown();
		self.states.shutdown();
	}

	#[tracing::instrument(name = "reactive", skip(self, context, epic), fields(application = context.identifier()))]
	pub async fn execute<E>(&self, context: Context, epic: E)
	where
		E: Epic<Action, State, Context>,
	{
		let output = epic.execute(self.actions.clone(), self.states.clone(), context);
		pin_mut!(output);
		while let Some(action) = output.next().await {
			self.actions.dispatch(action);
		}
	}
}
