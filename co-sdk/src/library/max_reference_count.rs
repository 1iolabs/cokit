// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_primitives::BlockSerializer;

/// Calculate max referecne count that will fint into an block.
///
/// [`cid::Cid`] size:
/// - Size depends on the used hash.
/// - The size of `Cid::default()` is 4 because there is no hash.
/// - Sha256 and blake3 got size of 32.
/// - [`cid::Cid`] min size for metadata is 4.
/// - Codec is a varint. So we add some extra space.
/// - This calculates with 40 bytes per [`cid::Cid`].
pub fn max_reference_count(max_block_size: usize) -> usize {
	// CID size
	let cid_size = BlockSerializer::default()
		.serialize(&0)
		.expect("BlockSerializer to serialize")
		.into_inner()
		.0
		.encoded_len();

	// CBOR size
	let cbor_size = 2;

	// Extra size
	let extra_size = 2;

	// result
	max_block_size / 2 / (cid_size + cbor_size + extra_size)
}
