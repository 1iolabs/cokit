pub trait StoreParams: std::fmt::Debug + Clone + Send + Sync + Unpin + 'static {
	const MAX_BLOCK_SIZE: usize;
}

#[derive(Debug, Clone)]
pub struct DefaultParams {}
impl StoreParams for DefaultParams {
	const MAX_BLOCK_SIZE: usize = 1_048_576;
}
