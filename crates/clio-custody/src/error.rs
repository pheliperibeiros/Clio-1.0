//! Erros do módulo de custódia.
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CustodyError {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialização JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Chave de selo não encontrada no chaveiro do sistema: {0}")]
    KeyNotFound(String),

    #[error("Erro de assinatura: {0}")]
    Signature(String),

    #[error("Verificação de integridade falhou: hash esperado {expected}, obtido {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("Arquivo vazio ou não legível: {0}")]
    EmptyFile(String),
}

pub type CustodyResult<T> = Result<T, CustodyError>;
