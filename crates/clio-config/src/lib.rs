//! clio-config — Persistência de configurações em TOML.
//!
//! Lê e grava `config.toml` de forma atômica (escreve em temp, fsync, renomeia).
//! Mantém backup rotativo dos últimos 5 arquivos.
//! Suporta modo portátil: se `portable.flag` existir ao lado do .exe, usa `./data/`.
#![forbid(unsafe_code)]
#![warn(clippy::all)]

pub mod backup;
pub mod error;
pub mod migrations;
pub mod schema;
pub mod storage;

pub use schema::AppConfig;
pub use storage::ConfigStorage;
