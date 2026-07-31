// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use dioxus::{
	dioxus_core::use_hook,
	hooks::{use_resource, Resource},
};
use std::{cell::RefCell, future::Future, rc::Rc};

/// Run a resource that belongs to a target and restart it whenever that target changes.
///
/// The resource is an ordinary [`use_resource`], so its own reactive dependencies keep working and
/// a rerun for an unchanged target keeps the value it already has while the next result is
/// pending. A target that differs from the one of the previous render restarts the future. The
/// restart cancels the run of the previous target, so that run can no longer complete into the
/// resource, and puts the resource back into its pending state before this render reads it.
///
/// The result of the previous target stays readable until the new run produced its own. Reach for
/// [`use_target_resource_cleared`] when the caller reads the resource value itself and must not be
/// served the result of the previous target.
pub(crate) fn use_target_resource<Target, T, F, Fut>(target: Target, future: F) -> Resource<T>
where
	Target: Clone + PartialEq + 'static,
	T: 'static,
	F: FnMut() -> Fut + 'static,
	Fut: Future<Output = T> + 'static,
{
	let mut resource = use_resource(future);
	if target_changed(target) {
		resource.restart();
	}
	resource
}

/// Run a resource that belongs to a target and drop its result whenever that target changes.
///
/// Restarts like [`use_target_resource`] and additionally clears the resource, so the render that
/// brought the new target already reads no result instead of the result of the previous one.
///
/// Clearing writes `None` into the value the resource holds, and that write is what wakes
/// everything reading it. `Resource::suspend` hands out a signal that maps this very value and
/// unwraps it, so every holder of such a signal is woken into reading a value that is gone. Only
/// use this variant where the caller keeps the resource itself. A caller that passes a signal
/// derived from the resource value to someone who outlives a target change needs
/// [`use_target_resource`].
pub(crate) fn use_target_resource_cleared<Target, T, F, Fut>(target: Target, future: F) -> Resource<T>
where
	Target: Clone + PartialEq + 'static,
	T: 'static,
	F: FnMut() -> Fut + 'static,
	Fut: Future<Output = T> + 'static,
{
	let mut resource = use_resource(future);
	if target_changed(target) {
		resource.clear();
		resource.restart();
	}
	resource
}

/// Remember the target across renders and report whether this render brought another one.
fn target_changed<Target: Clone + PartialEq + 'static>(target: Target) -> bool {
	let mounted = use_hook({
		let target = target.clone();
		move || Rc::new(RefCell::new(target))
	});

	// compare before the caller resets, so the reset still lands in the current render
	let mut mounted = mounted.borrow_mut();
	if *mounted == target {
		return false;
	}
	*mounted = target;
	true
}
