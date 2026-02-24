// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

pub trait StoreParams: std::fmt::Debug + Clone + Send + Sync + Unpin + 'static {
	const MAX_BLOCK_SIZE: usize;
}

#[derive(Debug, Clone)]
pub struct DefaultParams {}
impl StoreParams for DefaultParams {
	const MAX_BLOCK_SIZE: usize = 1_048_576;
}
