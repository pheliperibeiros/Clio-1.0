//! clio-store — SQLite: biblioteca de gravações, diário de eventos, rascunhos.
//!
//! Modo WAL + synchronous=FULL para durabilidade máxima.
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod db;
pub mod error;
pub mod library;
pub mod session_journal;

pub use db::Database;
pub use library::{GravacaoInfo, StatusGravacao};
pub use session_journal::SessionJournal;
