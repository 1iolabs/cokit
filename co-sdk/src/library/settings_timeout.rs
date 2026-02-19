// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

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
