// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	hooks::tests::support::{
		mount, multi_target, set_target, single_target, unresolvable_identity, Control, MultiTargetProps,
		SingleTargetProps, TestApplication,
	},
	Co,
};
use co_sdk::{BlockStorageExt, CoId};
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

#[co_test::timeout(10000)]
#[tokio::test]
async fn use_co_serves_the_new_target_in_the_switch_render() {
	let app = TestApplication::new("co-dioxus-use-co-switch").await;
	let missing = CoId::new("co-dioxus-missing-target");
	let known = app.create_co("known").await;

	let control = Control::default();
	let renders: Rc<RefCell<Vec<Co>>> = Default::default();
	let mut dom = mount(
		&app.context,
		single_target,
		SingleTargetProps { target: missing.clone(), control: control.clone(), renders: renders.clone() },
	);
	set_target(&mut dom, &control, known.clone());
	set_target(&mut dom, &control, known.clone());

	let rendered = renders.borrow().clone();
	assert_eq!(rendered.len(), 3, "one render per target write");
	assert_eq!(rendered[0].co(), missing);
	assert_eq!(rendered[1].co(), known, "the switch render already serves the new target");

	let detached = &rendered[0];
	let attached = &rendered[1];

	// the new target gets its own signals instead of inheriting the old ones
	assert_ne!(detached.reducer_state.id(), attached.reducer_state.id());
	assert_ne!(detached.last_error.id(), attached.last_error.id());

	// re-rendering the same target keeps the attachment it already has
	assert_eq!(rendered[2].reducer_state.id(), attached.reducer_state.id());
	assert!(!attached.handle.is_closed(), "the active actor survives a same target rerender");

	// the replaced actor shuts down although a public handle is still around
	detached.handle.closed().await.expect("replaced actor to close");
	assert!(
		matches!(detached.reducer_state.cloned(), Some(Err(_))),
		"the replaced attachment keeps the error of its own unknown CO"
	);

	// the new attachment starts clean and stays usable through its own storage handle
	assert!(!matches!(attached.reducer_state.cloned(), Some(Err(_))));
	assert!(attached.last_error.cloned().is_ok());
	attached.reducer_state().await.expect("reducer state of the new target");
	let cid = attached
		.storage()
		.set_serialized(&"switched".to_owned())
		.await
		.expect("write through the new storage handle");
	assert_eq!(
		attached
			.storage()
			.get_deserialized::<String>(&cid)
			.await
			.expect("read through the new storage handle"),
		"switched"
	);
}

#[co_test::timeout(10000)]
#[tokio::test]
async fn unmount_closes_the_actor_and_keeps_delayed_writes_valid() {
	let app = TestApplication::new("co-dioxus-use-co-unmount").await;
	let target = app.create_co("target").await;

	let control = Control::default();
	let renders: Rc<RefCell<Vec<Co>>> = Default::default();
	let dom = mount(
		&app.context,
		single_target,
		SingleTargetProps { target: target.clone(), control, renders: renders.clone() },
	);
	let mounted = renders.borrow()[0].clone();
	mounted.reducer_state().await.expect("reducer state of the mounted target");

	// hold the application queue so the failing push cannot report before the unmount
	let (release, released) = futures::channel::oneshot::channel::<()>();
	app.context.with_application(|_application| async move {
		released.await.ok();
	});
	mounted.dispatch(unresolvable_identity(), "core", "action".to_owned());

	drop(dom);
	mounted.handle.closed().await.expect("unmounted actor to close");

	// the delayed error still has somewhere to go
	release.send(()).expect("application queue to accept the release");
	app.context.ready().await.expect("application queue to drain");
	assert!(mounted.last_error.cloned().is_err(), "the delayed error lands on the unmounted attachment");
}

#[co_test::timeout(10000)]
#[tokio::test]
async fn delayed_error_stays_on_the_replaced_attachment() {
	let app = TestApplication::new("co-dioxus-use-co-delayed").await;
	let first = app.create_co("first").await;
	let second = app.create_co("second").await;

	let control = Control::default();
	let renders: Rc<RefCell<Vec<Co>>> = Default::default();
	let mut dom = mount(
		&app.context,
		single_target,
		SingleTargetProps { target: first.clone(), control: control.clone(), renders: renders.clone() },
	);
	let detached = renders.borrow()[0].clone();
	detached.reducer_state().await.expect("reducer state of the first target");

	// hold the application queue so the failing push cannot report before the switch
	let (release, released) = futures::channel::oneshot::channel::<()>();
	app.context.with_application(|_application| async move {
		released.await.ok();
	});
	detached.dispatch(unresolvable_identity(), "core", "action".to_owned());

	set_target(&mut dom, &control, second.clone());
	let attached = renders.borrow()[1].clone();
	assert_eq!(attached.co(), second);
	detached.handle.closed().await.expect("replaced actor to close");

	release.send(()).expect("application queue to accept the release");
	app.context.ready().await.expect("application queue to drain");
	assert!(detached.last_error.cloned().is_err(), "the delayed error lands on the replaced attachment");
	assert!(attached.last_error.cloned().is_ok(), "the active attachment stays free of the replaced error");
}

#[co_test::timeout(10000)]
#[tokio::test]
async fn use_cos_reconciles_order_and_duplicate_occurrences() {
	let app = TestApplication::new("co-dioxus-use-cos").await;
	let a = app.create_co("a").await;
	let b = app.create_co("b").await;
	let c = app.create_co("c").await;
	let d = app.create_co("d").await;

	let control = Control::default();
	let renders: Rc<RefCell<Vec<Vec<Co>>>> = Default::default();
	let mut dom = mount(
		&app.context,
		multi_target,
		MultiTargetProps {
			targets: vec![a.clone(), a.clone(), b.clone()],
			control: control.clone(),
			renders: renders.clone(),
		},
	);
	set_target(&mut dom, &control, vec![b.clone(), a.clone(), c.clone(), a.clone()]);
	set_target(&mut dom, &control, vec![d.clone(), c.clone()]);

	let rendered = renders.borrow().clone();
	assert_eq!(rendered.len(), 3, "one render per requested list");
	assert_eq!(targets(&rendered[0]), vec![a.clone(), a.clone(), b.clone()]);
	assert_eq!(targets(&rendered[1]), vec![b.clone(), a.clone(), c.clone(), a.clone()]);
	assert_eq!(targets(&rendered[2]), vec![d.clone(), c.clone()]);

	// every request takes the first still unused occurrence of that CO
	let (a1, a2, b1) = (&rendered[0][0], &rendered[0][1], &rendered[0][2]);
	assert_eq!(rendered[1][0].reducer_state.id(), b1.reducer_state.id());
	assert_eq!(rendered[1][1].reducer_state.id(), a1.reducer_state.id());
	assert_eq!(rendered[1][3].reducer_state.id(), a2.reducer_state.id());
	let c1 = &rendered[1][2];
	assert_ne!(c1.reducer_state.id(), a1.reducer_state.id());
	assert_eq!(rendered[2][1].reducer_state.id(), c1.reducer_state.id());
	let d1 = &rendered[2][0];

	// dropped occurrences shut down, retained ones keep working
	for dropped in [a1, a2, b1] {
		dropped.handle.closed().await.expect("dropped occurrence to close");
	}
	assert!(!c1.handle.is_closed(), "the retained occurrence stays open");
	c1.reducer_state().await.expect("reducer state of the retained occurrence");
	d1.reducer_state().await.expect("reducer state of the new occurrence");
}

fn targets(cos: &[Co]) -> Vec<CoId> {
	cos.iter().map(|co| co.co()).collect()
}
