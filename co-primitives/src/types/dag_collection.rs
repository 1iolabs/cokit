// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{Node, OptionLink};
use serde::{de::DeserializeOwned, Serialize};

/// Simple trait for creating a DagLink type object
pub trait DagCollection: Sized + Default {
	type Item: Clone + Serialize + DeserializeOwned + 'static;
	type Collection: Default + Clone + IntoIterator<Item = Self::Item> + FromIterator<Self::Item> + Extend<Self::Item>;

	fn link(&self) -> OptionLink<Node<Self::Item>>;
	fn set_link(&mut self, link: OptionLink<Node<Self::Item>>);
}
