// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::all_tags::all_tags;
use crate::CoContext;
use co_primitives::{CoId, CoTimeout};
use futures::StreamExt;
use std::{future::ready, time::Duration};

/// Get timeout setting from co tags.
///
/// See:
/// - [`co_core_co::Co::tags`]
/// - [`co_primitives::CoTimeout`]
pub async fn settings_timeout(context: &CoContext, co: &CoId, scope: Option<&str>) -> Duration {
	all_tags(context, co)
		.fold(CoTimeout::default_duration(), |result, tag| {
			ready(if let Ok(tag) = tag { CoTimeout::get_timeout([&tag], scope, Some(result)) } else { result })
		})
		.await
}
