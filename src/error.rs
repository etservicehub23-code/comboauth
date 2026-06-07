pub type Result<T> = std::result::Result<T, ComboAuthError>;

#[derive(Debug, thiserror::Error)]
pub enum ComboAuthError {
    #[error("terminal I/O failed")]
    Io(#[from] std::io::Error),
}
