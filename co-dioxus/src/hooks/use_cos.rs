// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{hooks::use_co::MountedCo, use_co_context, Co};
use co_sdk::CoId;
use dioxus::prelude::*;
use std::{cell::RefCell, ops::Deref, rc::Rc};

/// Use multiple COs at once.
///
/// The result keeps the requested order, a repeated id yields that many independent COs, and a CO
/// that leaves the list is detached.
pub fn use_cos(cos: ReadSignal<Vec<CoId>>) -> Cos {
	let context = use_co_context();
	let requested = cos();
	let mounted = use_hook(|| Rc::new(RefCell::new(Vec::<MountedCo>::new())));

	// reconcile before returning so the current render never sees a stale list
	let mut mounted = mounted.borrow_mut();
	let mut previous = std::mem::take(&mut *mounted);
	mounted.reserve(requested.len());
	for co_id in requested {
		let occurrence = match previous.iter().position(|previous| previous.co().co_id == co_id) {
			Some(index) => previous.remove(index),
			None => MountedCo::attach(&context, co_id),
		};
		mounted.push(occurrence);
	}

	// drop whatever is left as it was not requested again
	//  this will also run shutdown
	drop(previous);

	Cos(mounted.iter().map(|occurrence| occurrence.co().clone()).collect())
}

#[derive(Debug, Clone)]
pub struct Cos(Vec<Co>);

impl Deref for Cos {
	type Target = Vec<Co>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
