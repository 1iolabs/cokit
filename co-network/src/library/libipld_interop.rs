// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use co_primitives::Block as PrimitivesBlock;
use libp2p_bitswap::Block as BitswapBlock;

/// convert a [`BitswapBlock`] into a [`PrimitivesBlock`] (no hash verification).
pub fn from_bitswap_block(block: &BitswapBlock) -> PrimitivesBlock {
	PrimitivesBlock::new_unchecked(*block.cid(), block.data().to_vec())
}

/// convert a [`PrimitivesBlock`] into a [`BitswapBlock`] (no hash verification).
#[allow(dead_code)]
pub fn to_bitswap_block(block: PrimitivesBlock) -> BitswapBlock {
	let (cid, data) = block.into_inner();
	BitswapBlock::new_unchecked(cid, data)
}
