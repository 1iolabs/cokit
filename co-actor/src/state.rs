// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

/// State reducer.
///
/// # Abstraction Goals
/// - Separation of concerns: This is only intended to apply actions ot state.
/// - Make state changes trivialy testable.
/// - Deterministic state changes.
/// - Deterministic observe changes of interest (retuned actions).
pub trait Reducer<A>
where
	A: Clone + Send + 'static,
	Self: Sized,
{
	fn reduce(&mut self, action: A) -> Vec<A>;
}
