use thiserror::Error;

#[derive(Error, Debug)]
pub enum StateError {
    #[error("object not found")]
    NotFound,
    #[error("namespace not found")]
    NamespaceNotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("invalid request: {0}")]
    Invalid(String),
    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, StateError>;
