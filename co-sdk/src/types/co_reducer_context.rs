// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{CoReducer, CoStorage};
use async_trait::async_trait;
use std::{borrow::Cow, fmt::Debug, sync::Arc};

#[async_trait]
pub trait CoReducerContext: Debug {
	/// Get a new storage instance.
	///
	/// # Args
	/// - `force_local` - If true the new instance should not use networking.
	fn storage(&self, force_local: bool) -> CoStorage;

	/// Refresh reducer instance state from source.
	async fn refresh(&self, parent: CoReducer, co: CoReducer) -> anyhow::Result<()>;

	/// Clear reducer caches.
	async fn clear(&self, co: CoReducer);

	/// Test for a reducer feature.
	/// Features are always additive.
	///
	/// Known features:
	/// - `network` - Networking is enabled.
	/// - `encryption` - Storage encryption is enabled.
	fn has_feature(&self, feature: &CoReducerFeature<'_>) -> bool;
}

pub type CoReducerContextRef = Arc<dyn CoReducerContext + Send + Sync + 'static>;

#[non_exhaustive]
pub enum CoReducerFeature<'a> {
	Network,
	Encryption,
	Other(Cow<'a, str>),
}
