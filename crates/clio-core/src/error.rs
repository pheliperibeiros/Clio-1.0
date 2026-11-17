//! Tipos de erro do domínio clio-core.
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Nome de arquivo inválido: {0}")]
    InvalidFilename(String),

    #[error("Transição de estado inválida: {from:?} → {to:?}")]
    InvalidStateTransition {
        from: crate::recording::RecordingState,
        to: crate::recording::RecordingState,
    },

    #[error("Número de OAB inválido: '{0}' (esperado: letras, barra, estado e dígitos, ex.: OAB/TO 1234)")]
    InvalidOab(String),

    #[error("Campo obrigatório ausente: {0}")]
    MissingField(String),

    #[error("Valor fora do intervalo permitido: {field} = {value} (permitido: {min}–{max})")]
    OutOfRange {
        field: String,
        value: String,
        min: String,
        max: String,
    },
}

pub type CoreResult<T> = Result<T, CoreError>;
