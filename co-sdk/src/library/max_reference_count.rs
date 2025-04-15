use cid::Cid;

/// Calculate max referecne count that will fint into an block.
pub fn max_reference_count(max_block_size: usize) -> usize {
	max_block_size / 2 / Cid::default().encoded_len()
}
