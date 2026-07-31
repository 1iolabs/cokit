// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	hooks::tests::support::{
		collapsed, complete_run, gated_run, mount, pump, render_until, rendered, run_canceled, run_targets, set_target,
		Control, Log, Rendered, SelectorRuns, TestApplication,
	},
	use_co, use_co_reducer_state, use_cos, use_did_key_identity, use_selector, use_selector_state, use_selector_states,
	use_selectors, CoBlockStorage, CoSelector, CoSelectorState,
};
use co_sdk::{state, CoId, CoReducerState};
use dioxus::{
	dioxus_core::{DynamicNode, Properties, Template, TemplateNode, VComponent, VNode},
	prelude::*,
};
use std::{
	future::ready,
	panic::{catch_unwind, AssertUnwindSafe},
};

#[derive(Clone)]
struct SelectorProps {
	target: CoId,
	control: Control<CoId>,
	runs: SelectorRuns,
	renders: Log<Rendered<u8>>,
}

/// Render a gated [`use_selector`] and record what it exposed in every render.
fn selector_target(props: SelectorProps) -> Element {
	let target = use_signal(|| props.target.clone());
	use_hook(|| props.control.replace(Some(target)));
	let co = use_co(target.into());
	let resource = use_selector(&co, {
		let runs = props.runs.clone();
		let co_id = co.co();
		move |_storage: CoBlockStorage| gated_run(&runs, vec![co_id.clone()])
	});
	props.renders.borrow_mut().push(rendered(&resource));
	Ok(VNode::placeholder())
}

#[derive(Clone)]
struct MemoProps {
	target: CoId,
	control: Control<CoId>,
	runs: SelectorRuns,
	projections: Log<Option<u8>>,
}

/// Render a gated [`use_selector`] behind an ordinary memo projection and record that projection.
fn memo_target(props: MemoProps) -> Element {
	let target = use_signal(|| props.target.clone());
	use_hook(|| props.control.replace(Some(target)));
	let co = use_co(target.into());
	let resource = use_selector(&co, {
		let runs = props.runs.clone();
		let co_id = co.co();
		move |_storage: CoBlockStorage| gated_run(&runs, vec![co_id.clone()])
	});
	let projection = use_memo(move || resource.value().cloned().and_then(|result| result.ok()));
	props.projections.borrow_mut().push(projection.cloned());
	Ok(VNode::placeholder())
}

#[derive(Clone)]
struct RevisionProps {
	target: CoId,
	control: Control<u8>,
	runs: SelectorRuns,
	renders: Log<(u8, Rendered<u8>)>,
}

/// Render a gated [`use_selector`] whose future also depends on a revision the test can bump.
fn revision_target(props: RevisionProps) -> Element {
	let revision = use_signal(|| 0u8);
	use_hook(|| props.control.replace(Some(revision)));
	let target = use_signal(|| props.target.clone());
	let co = use_co(target.into());
	let resource = use_selector(&co, {
		let runs = props.runs.clone();
		let co_id = co.co();
		move |_storage: CoBlockStorage| {
			// reading the revision here makes it an ordinary reactive dependency of the resource
			let _revision = revision();
			gated_run(&runs, vec![co_id.clone()])
		}
	});
	props.renders.borrow_mut().push((revision(), rendered(&resource)));
	Ok(VNode::placeholder())
}

#[derive(Clone)]
struct SelectorStateProps {
	target: CoId,
	control: Control<CoId>,
	renders: Log<Rendered<CoId>>,
}

/// Render [`use_selector_state`] resolving to the CO it ran for and record every render.
fn selector_state_target(props: SelectorStateProps) -> Element {
	let target = use_signal(|| props.target.clone());
	use_hook(|| props.control.replace(Some(target)));
	let co = use_co(target.into());
	let resource = use_selector_state(&co, {
		let co_id = co.co();
		move |_storage: CoBlockStorage, _state: CoReducerState| ready(Ok(co_id.clone()))
	});
	props.renders.borrow_mut().push(rendered(&resource));
	Ok(VNode::placeholder())
}

#[derive(Clone)]
struct ReducerStateProps {
	target: CoId,
	control: Control<CoId>,
	renders: Log<Rendered<CoReducerState>>,
}

/// Render [`use_co_reducer_state`] and record every render.
fn reducer_state_target(props: ReducerStateProps) -> Element {
	let target = use_signal(|| props.target.clone());
	use_hook(|| props.control.replace(Some(target)));
	let co = use_co(target.into());
	let resource = use_co_reducer_state(&co);
	props.renders.borrow_mut().push(rendered(&resource));
	Ok(VNode::placeholder())
}

#[derive(Clone)]
struct SelectorsProps {
	targets: Vec<CoId>,
	control: Control<Vec<CoId>>,
	runs: SelectorRuns,
	renders: Log<Rendered<u8>>,
}

/// Render a gated [`use_selectors`] and record the COs of every run next to every render.
fn selectors_target(props: SelectorsProps) -> Element {
	let targets = use_signal(|| props.targets.clone());
	use_hook(|| props.control.replace(Some(targets)));
	let cos = use_cos(targets.into());
	let resource = use_selectors(&cos, {
		let runs = props.runs.clone();
		move |selectors: Vec<CoSelector>| {
			gated_run(&runs, selectors.iter().map(|selector| selector.co.clone()).collect())
		}
	});
	props.renders.borrow_mut().push(rendered(&resource));
	Ok(VNode::placeholder())
}

#[derive(Clone)]
struct SelectorStatesProps {
	targets: Vec<CoId>,
	control: Control<Vec<CoId>>,
	states: Log<Vec<(CoId, CoReducerState)>>,
	renders: Log<Rendered<Vec<CoId>>>,
}

/// Render [`use_selector_states`] resolving to the COs it ran for and record their ordered states.
fn selector_states_target(props: SelectorStatesProps) -> Element {
	let targets = use_signal(|| props.targets.clone());
	use_hook(|| props.control.replace(Some(targets)));
	let cos = use_cos(targets.into());
	let resource = use_selector_states(&cos, {
		let states = props.states.clone();
		move |selector_states: Vec<CoSelectorState>| {
			let ordered: Vec<(CoId, CoReducerState)> = selector_states
				.into_iter()
				.map(|selector| (selector.co, selector.state))
				.collect();
			let targets: Vec<CoId> = ordered.iter().map(|(co, _state)| co.clone()).collect();
			states.borrow_mut().push(ordered);
			ready(Ok(targets))
		}
	});
	props.renders.borrow_mut().push(rendered(&resource));
	Ok(VNode::placeholder())
}

/// Selector output without a [`PartialEq`] implementation.
#[derive(Clone)]
struct NotPartialEq(u8);

#[derive(Clone)]
struct NotPartialEqProps {
	target: CoId,
	renders: Log<Option<u8>>,
}

/// Render [`use_selector_state`] with an output that cannot be compared and record every render.
fn not_partial_eq_target(props: NotPartialEqProps) -> Element {
	let target = use_signal(|| props.target.clone());
	let co = use_co(target.into());
	let resource =
		use_selector_state(&co, |_storage: CoBlockStorage, _state: CoReducerState| ready(Ok(NotPartialEq(7))));
	let completed = resource.value().cloned().map(|result| result.expect("selector to succeed").0);
	props.renders.borrow_mut().push(completed);
	Ok(VNode::placeholder())
}

#[derive(Clone)]
struct IdentityProps {
	name: String,
	control: Control<String>,
	renders: Log<Option<String>>,
}

/// Render [`use_did_key_identity`] and record the name it exposed in every render.
fn identity_target(props: IdentityProps) -> Element {
	let name = use_signal(|| props.name.clone());
	use_hook(|| props.control.replace(Some(name)));
	let exposed = match use_did_key_identity(name()) {
		Ok(identity) => Some(identity.read().name.clone()),
		Err(_) => None,
	};
	props.renders.borrow_mut().push(exposed);
	Ok(VNode::placeholder())
}

#[co_test::timeout(10000)]
#[tokio::test]
async fn selector_switch_clears_a_completed_result() {
	let app = TestApplication::new("co-dioxus-selector-switch").await;
	let a = app.create_co("a").await;
	let b = app.create_co("b").await;
	let c = app.create_co("c").await;

	let control = Control::default();
	let runs = SelectorRuns::default();
	let renders: Log<Rendered<u8>> = Default::default();
	let mut dom = mount(
		&app.context,
		selector_target,
		SelectorProps { target: a.clone(), control: control.clone(), runs: runs.clone(), renders: renders.clone() },
	);
	pump(&mut dom);
	assert_eq!(run_targets(&runs), vec![vec![a.clone()]]);

	// a completed value is replaced by the loading state of the new target
	complete_run(&runs, 0, Ok(7)).expect("the first run to accept its value");
	render_until(&mut dom, || renders.borrow().last() == Some(&Some(Ok(7)))).await;
	let switch = renders.borrow().len();
	set_target(&mut dom, &control, b.clone());
	assert_eq!(renders.borrow()[switch], None, "the switch render clears the completed value");
	assert_eq!(run_targets(&runs), vec![vec![a.clone()], vec![b.clone()]]);

	// a completed error is replaced the same way
	pump(&mut dom);
	complete_run(&runs, 1, Err("selector failed".to_owned())).expect("the second run to accept its error");
	render_until(&mut dom, || renders.borrow().last() == Some(&Some(Err("selector failed".to_owned())))).await;
	let switch = renders.borrow().len();
	set_target(&mut dom, &control, c.clone());
	assert_eq!(renders.borrow()[switch], None, "the switch render clears the completed error");
	assert_eq!(run_targets(&runs), vec![vec![a], vec![b], vec![c]]);
}

#[co_test::timeout(10000)]
#[tokio::test]
async fn selector_switch_cancels_the_run_of_the_previous_target() {
	let app = TestApplication::new("co-dioxus-selector-cancel").await;
	let a = app.create_co("a").await;
	let b = app.create_co("b").await;

	let control = Control::default();
	let runs = SelectorRuns::default();
	let renders: Log<Rendered<u8>> = Default::default();
	let mut dom = mount(
		&app.context,
		selector_target,
		SelectorProps { target: a.clone(), control: control.clone(), runs: runs.clone(), renders: renders.clone() },
	);
	pump(&mut dom);

	let switch = renders.borrow().len();
	set_target(&mut dom, &control, b.clone());
	assert_eq!(renders.borrow()[switch], None, "the switch render clears the pending state");
	assert_eq!(run_targets(&runs), vec![vec![a], vec![b]]);
	assert!(run_canceled(&runs, 0), "the still running future of the previous target is dropped");

	pump(&mut dom);
	complete_run(&runs, 1, Ok(9)).expect("the run of the new target to accept its value");
	render_until(&mut dom, || renders.borrow().last() == Some(&Some(Ok(9)))).await;

	// the late future of the previous target has nowhere left to write
	assert_eq!(complete_run(&runs, 0, Ok(1)), Err(Ok(1)));
	pump(&mut dom);
	assert_eq!(renders.borrow().last(), Some(&Some(Ok(9))), "the value of the new target survives");
}

#[co_test::timeout(10000)]
#[tokio::test]
async fn reducer_state_restarts_for_the_new_target() {
	let app = TestApplication::new("co-dioxus-reducer-state-switch").await;
	let a = app.create_co("a").await;
	let b = app.create_co("b").await;

	let control = Control::default();
	let renders: Log<Rendered<CoReducerState>> = Default::default();
	let mut dom = mount(
		&app.context,
		reducer_state_target,
		ReducerStateProps { target: a, control: control.clone(), renders: renders.clone() },
	);
	render_until(&mut dom, || matches!(renders.borrow().last(), Some(Some(Ok(_))))).await;
	let first = completed(&renders);

	let switch = renders.borrow().len();
	set_target(&mut dom, &control, b);
	assert_eq!(renders.borrow()[switch], None, "the switch render clears the state of the previous CO");

	render_until(&mut dom, || matches!(renders.borrow().last(), Some(Some(Ok(_))))).await;
	assert_ne!(completed(&renders), first, "the restarted resource reports the state of the new CO");
}

/// The value the last render of a reducer state resource exposed.
fn completed(renders: &Log<Rendered<CoReducerState>>) -> CoReducerState {
	renders
		.borrow()
		.last()
		.cloned()
		.flatten()
		.expect("a completed render")
		.expect("the reducer state to be read")
}

#[co_test::timeout(10000)]
#[tokio::test]
async fn selector_state_restarts_for_the_new_target() {
	let app = TestApplication::new("co-dioxus-selector-state-switch").await;
	let a = app.create_co("a").await;
	let b = app.create_co("b").await;

	let control = Control::default();
	let renders: Log<Rendered<CoId>> = Default::default();
	let mut dom = mount(
		&app.context,
		selector_state_target,
		SelectorStateProps { target: a.clone(), control: control.clone(), renders: renders.clone() },
	);
	render_until(&mut dom, || renders.borrow().last() == Some(&Some(Ok(a.clone())))).await;

	let switch = renders.borrow().len();
	set_target(&mut dom, &control, b.clone());
	assert_eq!(renders.borrow()[switch], None, "the switch render clears the value of the previous CO");

	render_until(&mut dom, || renders.borrow().last() == Some(&Some(Ok(b.clone())))).await;
	assert!(
		!renders.borrow()[switch..].contains(&Some(Ok(a))),
		"no render after the switch exposes the value of the previous CO"
	);
}

#[co_test::timeout(10000)]
#[tokio::test]
async fn selectors_restart_in_the_requested_order() {
	let app = TestApplication::new("co-dioxus-selectors-order").await;
	let a = app.create_co("a").await;
	let b = app.create_co("b").await;

	let control = Control::default();
	let runs = SelectorRuns::default();
	let renders: Log<Rendered<u8>> = Default::default();
	let mut dom = mount(
		&app.context,
		selectors_target,
		SelectorsProps {
			targets: vec![a.clone(), b.clone()],
			control: control.clone(),
			runs: runs.clone(),
			renders: renders.clone(),
		},
	);
	pump(&mut dom);
	complete_run(&runs, 0, Ok(1)).expect("the first run to accept its value");
	render_until(&mut dom, || renders.borrow().last() == Some(&Some(Ok(1)))).await;

	// the same COs in another order are another target
	let switch = renders.borrow().len();
	set_target(&mut dom, &control, vec![b.clone(), a.clone()]);
	assert_eq!(renders.borrow()[switch], None, "the reorder render clears the previous value");
	assert_eq!(run_targets(&runs), vec![vec![a.clone(), b.clone()], vec![b, a]], "every run keeps the requested order");

	pump(&mut dom);
	complete_run(&runs, 1, Ok(2)).expect("the second run to accept its value");
	render_until(&mut dom, || renders.borrow().last() == Some(&Some(Ok(2)))).await;
}

#[co_test::timeout(10000)]
#[tokio::test]
async fn selector_states_follow_added_removed_and_replaced_targets() {
	let app = TestApplication::new("co-dioxus-selector-states").await;
	let a = app.create_co("a").await;
	let b = app.create_co("b").await;
	let c = app.create_co("c").await;
	let d = app.create_co("d").await;

	let control = Control::default();
	let states: Log<Vec<(CoId, CoReducerState)>> = Default::default();
	let renders: Log<Rendered<Vec<CoId>>> = Default::default();
	let mut dom = mount(
		&app.context,
		selector_states_target,
		SelectorStatesProps {
			targets: vec![a.clone(), b.clone()],
			control: control.clone(),
			states: states.clone(),
			renders: renders.clone(),
		},
	);
	let requested = [
		vec![a.clone(), b.clone()],
		vec![a.clone(), b.clone(), c.clone()],
		vec![a.clone(), c.clone()],
		vec![d.clone(), c.clone()],
	];
	render_until(&mut dom, || renders.borrow().last() == Some(&Some(Ok(requested[0].clone())))).await;
	for targets in &requested[1..] {
		let switch = renders.borrow().len();
		set_target(&mut dom, &control, targets.clone());
		assert_eq!(renders.borrow()[switch], None, "the switch render clears the value of the previous targets");
		render_until(&mut dom, || renders.borrow().last() == Some(&Some(Ok(targets.clone())))).await;
	}

	// every run saw the requested COs in the requested order, each with its own state
	let ordered: Vec<Vec<CoId>> = states
		.borrow()
		.iter()
		.map(|run| run.iter().map(|(co, _state)| co.clone()).collect())
		.collect();
	assert_eq!(collapsed(&ordered), requested.to_vec());
	for run in states.borrow().iter() {
		for (co, state) in run {
			assert!(
				run.iter()
					.all(|(other_co, other_state)| (co == other_co) == (state == other_state)),
				"every CO of a run contributes its own state"
			);
		}
	}
}

#[co_test::timeout(10000)]
#[tokio::test]
async fn memo_projection_observes_the_switch_reset() {
	let app = TestApplication::new("co-dioxus-selector-memo").await;
	let a = app.create_co("a").await;
	let b = app.create_co("b").await;

	let control = Control::default();
	let runs = SelectorRuns::default();
	let projections: Log<Option<u8>> = Default::default();
	let mut dom = mount(
		&app.context,
		memo_target,
		MemoProps { target: a.clone(), control: control.clone(), runs: runs.clone(), projections: projections.clone() },
	);
	pump(&mut dom);
	complete_run(&runs, 0, Ok(7)).expect("the first run to accept its value");
	render_until(&mut dom, || projections.borrow().last() == Some(&Some(7))).await;

	// the new target produces the same output, so only the reset makes the switch visible
	set_target(&mut dom, &control, b.clone());
	pump(&mut dom);
	complete_run(&runs, 1, Ok(7)).expect("the second run to accept its value");
	render_until(&mut dom, || projections.borrow().last() == Some(&Some(7))).await;

	assert_eq!(collapsed(&projections.borrow()), vec![None, Some(7), None, Some(7)]);
	assert_eq!(run_targets(&runs), vec![vec![a], vec![b]]);
}

#[co_test::timeout(10000)]
#[tokio::test]
async fn unchanged_target_keeps_its_value_while_the_next_run_is_pending() {
	let app = TestApplication::new("co-dioxus-selector-revision").await;
	let a = app.create_co("a").await;

	let control = Control::default();
	let runs = SelectorRuns::default();
	let renders: Log<(u8, Rendered<u8>)> = Default::default();
	let mut dom = mount(
		&app.context,
		revision_target,
		RevisionProps { target: a.clone(), control: control.clone(), runs: runs.clone(), renders: renders.clone() },
	);
	pump(&mut dom);
	complete_run(&runs, 0, Ok(7)).expect("the first run to accept its value");
	render_until(&mut dom, || renders.borrow().last().map(|(_, value)| value.clone()) == Some(Some(Ok(7)))).await;

	// bumping an ordinary reactive dependency reruns the future without touching the target
	let settled = renders.borrow().len();
	set_target(&mut dom, &control, 1);
	render_until(&mut dom, || run_targets(&runs).len() == 2 && renders.borrow().len() > settled).await;
	assert_eq!(run_targets(&runs), vec![vec![a.clone()], vec![a]]);
	assert!(
		renders.borrow()[settled..].iter().all(|(_, value)| value == &Some(Ok(7))),
		"the unchanged target keeps its completed value while the next result is pending"
	);

	complete_run(&runs, 1, Ok(9)).expect("the second run to accept its value");
	render_until(&mut dom, || renders.borrow().last().map(|(_, value)| value.clone()) == Some(Some(Ok(9)))).await;
}

#[co_test::timeout(10000)]
#[tokio::test]
async fn selector_state_completes_with_an_output_that_is_not_comparable() {
	let app = TestApplication::new("co-dioxus-selector-not-partial-eq").await;
	let a = app.create_co("a").await;

	let renders: Log<Option<u8>> = Default::default();
	let mut dom = mount(&app.context, not_partial_eq_target, NotPartialEqProps { target: a, renders: renders.clone() });
	render_until(&mut dom, || renders.borrow().last() == Some(&Some(7))).await;
}

#[derive(Clone)]
struct IdentityChildProps {
	identity: ReadSignal<state::Identity>,
	reads: Log<Result<String, String>>,
}
impl Properties for IdentityChildProps {
	type Builder = ();

	fn builder() -> Self::Builder {}

	fn memoize(&mut self, other: &Self) -> bool {
		self.clone_from(other);
		false
	}
}

/// Read an identity that was handed down as a prop and record what every read produced.
fn identity_child(props: IdentityChildProps) -> Element {
	let read = catch_unwind(AssertUnwindSafe(|| props.identity.read().name.clone())).map_err(|_| "PANIC".to_owned());
	props.reads.borrow_mut().push(read);
	Ok(VNode::placeholder())
}

#[derive(Clone)]
struct IdentityParentProps {
	name: String,
	control: Control<String>,
	renders: Log<Option<String>>,
	reads: Log<Result<String, String>>,
}

/// Hand the identity down to a child that keeps reading it while the name changes.
fn identity_parent(props: IdentityParentProps) -> Element {
	static TEMPLATE: Template =
		Template { roots: &[TemplateNode::Dynamic { id: 0usize }], node_paths: &[&[0u8]], attr_paths: &[] };

	let name = use_signal(|| props.name.clone());
	use_hook(|| props.control.replace(Some(name)));
	match use_did_key_identity(name()) {
		Ok(identity) => {
			props.renders.borrow_mut().push(Some(identity.read().name.clone()));
			let child = IdentityChildProps { identity, reads: props.reads.clone() };
			Ok(VNode::new(
				None,
				TEMPLATE,
				Box::new([DynamicNode::Component(VComponent::new(identity_child, child, "identity_child"))]),
				Box::new([]),
			))
		},
		Err(err) => {
			props.renders.borrow_mut().push(None);
			Err(err)
		},
	}
}

#[co_test::timeout(10000)]
#[tokio::test]
async fn identity_switch_keeps_a_handed_down_signal_readable() {
	let app = TestApplication::new("co-dioxus-identity-child").await;

	let control = Control::default();
	let renders: Log<Option<String>> = Default::default();
	let reads: Log<Result<String, String>> = Default::default();
	let mut dom = mount(
		&app.context,
		identity_parent,
		IdentityParentProps {
			name: "alpha".to_owned(),
			control: control.clone(),
			renders: renders.clone(),
			reads: reads.clone(),
		},
	);
	render_until(&mut dom, || reads.borrow().last() == Some(&Ok("alpha".to_owned()))).await;

	// the component that owns the lookup suspends again for the new name
	let switch = renders.borrow().len();
	set_target(&mut dom, &control, "beta".to_owned());
	assert_eq!(renders.borrow()[switch], None, "the switch render suspends again");

	// the child that was handed the identity settles on the new one
	render_until(&mut dom, || reads.borrow().last() == Some(&Ok("beta".to_owned()))).await;
	assert!(
		reads.borrow().iter().all(|read| read.is_ok()),
		"every read of the handed down identity succeeds, got {:?}",
		reads.borrow()
	);
}

#[co_test::timeout(10000)]
#[tokio::test]
async fn identity_switch_exposes_only_the_new_name() {
	let app = TestApplication::new("co-dioxus-identity-switch").await;

	let control = Control::default();
	let renders: Log<Option<String>> = Default::default();
	let mut dom = mount(
		&app.context,
		identity_target,
		IdentityProps { name: "alpha".to_owned(), control: control.clone(), renders: renders.clone() },
	);
	render_until(&mut dom, || renders.borrow().last() == Some(&Some("alpha".to_owned()))).await;

	let switch = renders.borrow().len();
	set_target(&mut dom, &control, "beta".to_owned());
	assert_eq!(renders.borrow()[switch], None, "the switch render stops exposing the previous identity");

	render_until(&mut dom, || renders.borrow().last() == Some(&Some("beta".to_owned()))).await;
	assert!(
		!renders.borrow()[switch..].contains(&Some("alpha".to_owned())),
		"no render after the switch exposes the previous identity"
	);
}
