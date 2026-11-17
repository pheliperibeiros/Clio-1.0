//! clio-core — Tipos de domínio puro, máquina de estados, validações.
//!
//! Sem I/O, sem UI, sem GStreamer. Apenas lógica pura e testável.
#![forbid(unsafe_code)]
#![warn(clippy::all)]

pub mod error;
pub mod filename;
pub mod models;
pub mod recording;
pub mod session;
pub mod types;
pub mod validation;
