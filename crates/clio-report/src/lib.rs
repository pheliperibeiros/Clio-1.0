//! clio-report — Renderização de modelos, PDF, DOCX e DOTX.
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod context;
pub mod docx;
pub mod error;
pub mod filters;
pub mod pdf;
pub mod template;

pub use context::ReportContext;
pub use template::{TemplateEngine, TipoDocumento};
