use crate::co_v1::c_ssize_t;

pub fn result_size(result: c_ssize_t) -> usize {
	result.try_into().expect("positive result")
}
