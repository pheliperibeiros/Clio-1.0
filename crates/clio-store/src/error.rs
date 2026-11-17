//! Erros do módulo de armazenamento.
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Serialização JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("Sessão não encontrada: {0}")]
    SessionNotFound(String),

    #[error("Gravação não encontrada: {0}")]
    RecordingNotFound(String),

    #[error("Banco de dados em estado inconsistente: {0}")]
    InconsistentState(String),
}

pub type StoreResult<T> = Result<T, StoreError>;
