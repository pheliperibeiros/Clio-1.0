//! Erros do módulo de configuração.
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Falha de I/O ao acessar configuração: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erro de serialização TOML: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("Erro de desserialização TOML: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    #[error("Configuração corrompida; backup restaurado de: {path}")]
    CorruptedConfig { path: String },

    #[error("Nenhum backup válido encontrado após arquivo corrompido")]
    NoValidBackup,

    #[error("Versão de esquema desconhecida: {0}")]
    UnknownSchemaVersion(u32),

    #[error("Falha na migração de v{from} para v{to}: {reason}")]
    MigrationFailed { from: u32, to: u32, reason: String },

    #[error("Valor fora do intervalo em '{field}': {value}. Corrigido para o padrão.")]
    OutOfRangeCorrected { field: String, value: String },

    #[error("Arquivo de configuração importado inválido: {0}")]
    InvalidImport(String),
}

pub type ConfigResult<T> = Result<T, ConfigError>;
