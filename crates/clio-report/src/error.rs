//! Erros do módulo de relatórios.
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReportError {
    #[error("Renderização de modelo: {0}")]
    Template(String),

    #[error("Variável desconhecida '{var}' na linha {line}")]
    UnknownVariable { var: String, line: usize },

    #[error("Sintaxe inválida na linha {line}: {msg}")]
    InvalidSyntax { line: usize, msg: String },

    #[error("Profundidade de laço excedida (máximo: {max})")]
    LoopDepthExceeded { max: usize },

    #[error("Modelo excede o tamanho máximo permitido ({max_bytes} bytes)")]
    TemplateTooLarge { max_bytes: usize },

    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("PDF: {0}")]
    Pdf(String),

    #[error("DOCX/ZIP: {0}")]
    Docx(String),

    #[error("Imagem de cabeçalho inválida: {0}")]
    InvalidHeader(String),

    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
}

pub type ReportResult<T> = Result<T, ReportError>;
