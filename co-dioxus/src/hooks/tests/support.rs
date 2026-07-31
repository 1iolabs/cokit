// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{use_co, use_cos, Co, CoContext, CoError};
use co_sdk::{state::Identity, Application, ApplicationBuilder, CoId, CoStorageSetting, CreateCo};
use dioxus::{
	dioxus_core::{ComponentFunction, NoOpMutations, VNode, VirtualDom},
	hooks::Resource,
	prelude::*,
};
use futures::channel::oneshot;
use std::{
	cell::RefCell,
	future::{poll_fn, Future},
	rc::Rc,
	task::Poll,
};

/// Memory backed application together with the dioxus context that drives it.
pub(crate) struct TestApplication {
	pub application: Application,
	pub context: CoContext,
}
impl TestApplication {
	pub(crate) async fn new(name: &str) -> Self {
		let application =
			ApplicationBuilder::new_with_storage(co_test::test_application_identifier(name), CoStorageSetting::Memory)
				.without_keychain()
				.build()
				.await
				.expect("application to build");
		let context = CoContext::new_application(application.clone());
		Self { application, context }
	}

	/// Create a CO owned by the local device identity and return its id.
	pub(crate) async fn create_co(&self, name: &str) -> CoId {
		let identity = self.application.local_identity();
		self.application
			.create_co(identity, CreateCo::generate(name.to_owned()))
			.await
			.expect("co to be created")
			.id()
			.clone()
	}
}

/// Signal a component publishes on its first render so the test can drive later renders.
pub(crate) type Control<T> = Rc<RefCell<Option<Signal<T>>>>;

/// Ordered record a component appends to on every render.
pub(crate) type Log<T> = Rc<RefCell<Vec<T>>>;

#[derive(Clone)]
pub(crate) struct SingleTargetProps {
	pub target: CoId,
	pub control: Control<CoId>,
	pub renders: Rc<RefCell<Vec<Co>>>,
}

/// Render [`use_co`] and record what it returned for every render.
pub(crate) fn single_target(props: SingleTargetProps) -> Element {
	let target = use_signal(|| props.target.clone());
	use_hook(|| props.control.replace(Some(target)));
	let co = use_co(target.into());
	props.renders.borrow_mut().push(co);
	Ok(VNode::placeholder())
}

#[derive(Clone)]
pub(crate) struct MultiTargetProps {
	pub targets: Vec<CoId>,
	pub control: Control<Vec<CoId>>,
	pub renders: Rc<RefCell<Vec<Vec<Co>>>>,
}

/// Render [`use_cos`] and record what it returned for every render.
pub(crate) fn multi_target(props: MultiTargetProps) -> Element {
	let targets = use_signal(|| props.targets.clone());
	use_hook(|| props.control.replace(Some(targets)));
	let cos = use_cos(targets.into());
	props.renders.borrow_mut().push(cos.to_vec());
	Ok(VNode::placeholder())
}

/// Mount a component with the [`CoContext`] available to its hooks and run the first render.
pub(crate) fn mount<P, M>(context: &CoContext, component: impl ComponentFunction<P, M>, props: P) -> VirtualDom
where
	P: Clone + 'static,
	M: 'static,
{
	let mut dom = VirtualDom::new_with_props(component, props);
	dom.provide_root_context(context.clone());
	dom.rebuild_in_place();
	dom
}

/// Publish a new target through the component control and render the result.
pub(crate) fn set_target<T: 'static>(dom: &mut VirtualDom, control: &Control<T>, target: T) {
	let mut control = (*control.borrow()).expect("component to publish its control signal");
	dom.in_runtime(move || control.set(target));
	dom.render_immediate(&mut NoOpMutations);
}

/// Run every render and every dioxus task that is ready right now, without waiting for new work.
pub(crate) fn pump(dom: &mut VirtualDom) {
	dom.render_immediate(&mut NoOpMutations);
}

/// Renders a single [`render_until`] drives before it reports the awaited state as unreachable.
const RENDER_BUDGET: usize = 1000;

/// Drive the dom until `ready` reports that the state the test waits for arrived.
///
/// Rendering and waiting for work both finish without suspending whenever the dom still has
/// something to do, so the loop returns to the runtime once per turn. Without that a dom that
/// never runs out of work would occupy the test task, and the test timeout, which is only checked
/// while the test suspends, would never be reached. The budget turns a state that never arrives
/// into a failure with a message instead of a run that goes on forever.
pub(crate) async fn render_until(dom: &mut VirtualDom, mut ready: impl FnMut() -> bool) {
	for _ in 0..RENDER_BUDGET {
		dom.render_immediate(&mut NoOpMutations);
		if ready() {
			return;
		}
		yield_once().await;
		dom.wait_for_work().await;
	}
	panic!("the awaited state did not arrive within {RENDER_BUDGET} renders");
}

/// Return to the runtime exactly once.
async fn yield_once() {
	let mut suspend = true;
	poll_fn(move |context| {
		if suspend {
			suspend = false;
			context.waker().wake_by_ref();
			Poll::Pending
		} else {
			Poll::Ready(())
		}
	})
	.await;
}

/// Drop repetitions so a log shows the sequence of values a component actually went through.
pub(crate) fn collapsed<T: Clone + PartialEq>(values: &[T]) -> Vec<T> {
	let mut sequence: Vec<T> = Vec::new();
	for value in values {
		if sequence.last() != Some(value) {
			sequence.push(value.clone());
		}
	}
	sequence
}

/// Identity no application can resolve, so every push made with it fails.
pub(crate) fn unresolvable_identity() -> Identity {
	Identity {
		did: "did:key:z0000000000000000000000000000000000000000000".to_owned(),
		name: "unresolvable".to_owned(),
		description: String::new(),
	}
}

/// What a resource exposed in one render, with the error reduced to its message.
pub(crate) type Rendered<T> = Option<Result<T, String>>;

/// Read a resource the way a component does and reduce it to a comparable value.
pub(crate) fn rendered<T: Clone + 'static>(resource: &Resource<Result<T, CoError>>) -> Rendered<T> {
	resource
		.value()
		.cloned()
		.map(|result| result.map_err(|error| error.to_string()))
}

/// Result a gated selector run is completed with.
pub(crate) type RunResult = Result<u8, String>;

/// Ordered log of the runs a selector hook started.
pub(crate) type SelectorRuns = Rc<RefCell<Vec<SelectorRun>>>;

/// One selector run, recorded together with the COs it was started for.
pub(crate) struct SelectorRun {
	targets: Vec<CoId>,
	sender: Option<oneshot::Sender<RunResult>>,
}

/// Record a run for `targets` and return the future the test completes through [`complete_run`].
pub(crate) fn gated_run(runs: &SelectorRuns, targets: Vec<CoId>) -> impl Future<Output = Result<u8, anyhow::Error>> {
	let (sender, receiver) = oneshot::channel();
	runs.borrow_mut().push(SelectorRun { targets, sender: Some(sender) });
	async move {
		match receiver.await? {
			Ok(value) => Ok(value),
			Err(message) => Err(anyhow::anyhow!(message)),
		}
	}
}

/// Complete a recorded run, reporting the payload back when its future was already dropped.
pub(crate) fn complete_run(runs: &SelectorRuns, index: usize, result: RunResult) -> Result<(), RunResult> {
	let sender = runs.borrow_mut()[index].sender.take().expect("run to be completed once");
	sender.send(result)
}

/// The COs every recorded run was started for, in run order.
pub(crate) fn run_targets(runs: &SelectorRuns) -> Vec<Vec<CoId>> {
	runs.borrow().iter().map(|run| run.targets.clone()).collect()
}

/// Whether the future of a recorded run was dropped before the run was completed.
pub(crate) fn run_canceled(runs: &SelectorRuns, index: usize) -> bool {
	runs.borrow()[index].sender.as_ref().is_some_and(|sender| sender.is_canceled())
}
