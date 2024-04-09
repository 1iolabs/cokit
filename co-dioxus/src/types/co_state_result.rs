use libipld::Cid;

#[derive(Debug, Clone, PartialEq)]
pub enum CoStateResult<T> {
	Pending,
	State(Option<Cid>, T),
	Error(String),
}
