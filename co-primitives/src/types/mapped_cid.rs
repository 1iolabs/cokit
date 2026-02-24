// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use anyhow::anyhow;
use cid::Cid;
use derive_more::From;
use serde::{Deserialize, Serialize};

/// Simple type to make mapped Cid's clear.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, From, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OptionMappedCid {
	/// Unmapped Cid (Internal/Mapped is the same as External/Plain).
	Unmapped(Cid),

	/// Mapped Cid (Internal/Mapped, External/Plain).
	Mapped(MappedCid),
}
impl OptionMappedCid {
	pub fn new(internal: Cid, external: Cid) -> OptionMappedCid {
		MappedCid(internal, external).into()
	}

	pub fn new_unmapped(internal: Cid) -> OptionMappedCid {
		internal.into()
	}

	pub fn mapped(&self) -> Option<MappedCid> {
		match self {
			OptionMappedCid::Unmapped(_cid) => None,
			OptionMappedCid::Mapped(mapped_cid) => Some(*mapped_cid),
		}
	}

	pub fn external(&self) -> Cid {
		match self {
			OptionMappedCid::Unmapped(cid) => *cid,
			OptionMappedCid::Mapped(MappedCid(_internal, external)) => *external,
		}
	}

	pub fn force_external(&self) -> Result<Cid, anyhow::Error> {
		match self {
			OptionMappedCid::Unmapped(cid) => Err(anyhow!("failed to map: {:?}", cid)),
			OptionMappedCid::Mapped(MappedCid(_internal, external)) => Ok(*external),
		}
	}

	pub fn internal(&self) -> Cid {
		match self {
			OptionMappedCid::Unmapped(cid) => *cid,
			OptionMappedCid::Mapped(MappedCid(internal, _external)) => *internal,
		}
	}
}

/// Mapped Cid.
///
/// # Args
/// - 0: Internal/Mapped.
/// - 1: External/Plain.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MappedCid(pub Cid, pub Cid);
impl MappedCid {
	pub fn new(internal: Cid, external: Cid) -> Self {
		Self(internal, external)
	}

	pub fn external(&self) -> Cid {
		self.1
	}

	pub fn internal(&self) -> Cid {
		self.0
	}
}
impl From<(Cid, Cid)> for MappedCid {
	fn from((internal, external): (Cid, Cid)) -> Self {
		Self(internal, external)
	}
}
