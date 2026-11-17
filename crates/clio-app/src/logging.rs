//! Inicialização de logs (tracing + tracing-appender com rotação).

use anyhow::Result;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub fn init_logging() -> Result<WorkerGuard> {
    let log_dir = clio_config::storage::data_dir().join("logs");
    std::fs::create_dir_all(&log_dir)?;

    // Rotação diária dos logs
    let file_appender = tracing_appender::rolling::daily(&log_dir, "clio.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Nível padrão: info (sem dados pessoais em info)
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("clio=info,warn"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true)
                .with_thread_names(true),
        )
        .with(
            // Console apenas em debug builds
            fmt::layer()
                .with_writer(std::io::stderr)
                .with_target(false)
                .with_level(true),
        )
        .init();

    Ok(guard)
}
