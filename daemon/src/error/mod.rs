#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("Something went wrong")]
    Unknown(#[from] anyhow::Error),
}
