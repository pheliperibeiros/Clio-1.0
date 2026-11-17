//! clio-app — ponto de entrada do aplicativo.
//!
//! Responsável por:
//! - Instância única (single-instance lock)
//! - Inicialização de logs (tracing)
//! - AppUserModelID no Windows
//! - Ícone da janela
//! - Verificação de sessões sem encerramento (recuperação)
//! - Lançamento da UI (eframe/egui)

#![deny(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod app_state;
mod instance;
mod logging;
mod ui;

use anyhow::Context;
use tracing::info;

/// Versão do aplicativo (injetada via Cargo.toml).
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Hash do commit (injetado em build.rs).
pub const APP_BUILD: &str = env!("CLIO_BUILD_HASH");

fn main() -> anyhow::Result<()> {
    // ── Instância única ───────────────────────────────────────────────────────
    let _lock = instance::SingleInstanceLock::acquire()
        .context("Outra instância do Clio já está em execução.")?;

    // ── Logs ──────────────────────────────────────────────────────────────────
    let _log_guard = logging::init_logging().context("Falha ao inicializar logs")?;
    info!("Clio {} (build {}) iniciando", APP_VERSION, APP_BUILD);

    // ── AppUserModelID (Windows — agrupamento na barra de tarefas) ────────────
    #[cfg(target_os = "windows")]
    set_app_user_model_id();

    // ── Carregar configuração ─────────────────────────────────────────────────
    let config_storage = clio_config::ConfigStorage::new();
    let (config, config_warnings) = config_storage.carregar().unwrap_or_else(|e| {
        tracing::error!("Falha ao carregar configuração: {e}; usando padrão");
        (clio_config::AppConfig::default(), vec![e.to_string()])
    });

    // ── Abrir banco de dados ──────────────────────────────────────────────────
    let db_path = clio_config::storage::data_dir().join("clio.db");
    let db = clio_store::Database::open(&db_path).context("Falha ao abrir banco de dados")?;

    // ── Verificar sessões pendentes (recuperação) ─────────────────────────────
    let sessoes_pendentes = db.sessoes_pendentes().unwrap_or_default();
    if !sessoes_pendentes.is_empty() {
        info!(
            "{} sessão(ões) sem encerramento regular detectada(s)",
            sessoes_pendentes.len()
        );
    }

    // ── Ícone da janela ───────────────────────────────────────────────────────
    let icon = carregar_icone_janela();

    // ── Iniciar UI ────────────────────────────────────────────────────────────
    let native_opts = eframe::NativeOptions {
        viewport: {
            let mut vp = egui::ViewportBuilder::default()
                .with_title("Clio — Gravador de Oitivas e Audiências")
                .with_min_inner_size([1024.0, 600.0])
                .with_inner_size([
                    config.janela.width.unwrap_or(1366.0),
                    config.janela.height.unwrap_or(768.0),
                ])
                .with_maximized(config.janela.maximized);

            if let Some(icon) = icon {
                vp = vp.with_icon(icon);
            }
            vp
        },
        ..Default::default()
    };

    let initial_state = app_state::AppState::new(
        config,
        config_storage,
        config_warnings,
        sessoes_pendentes,
        db,
    );

    eframe::run_native(
        "Clio — Gravador de Oitivas e Audiências",
        native_opts,
        Box::new(|cc| {
            // Registrar fontes extras (Noto Sans para acentuação)
            setup_fonts(&cc.egui_ctx);
            // Tema escuro sóbrio
            cc.egui_ctx.set_visuals(dark_visuals());
            Ok(Box::new(app::ClioApp::new(cc, initial_state)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Falha na UI: {e}"))
}

/// Carrega o ícone da janela a partir do icon.ico embutido.
fn carregar_icone_janela() -> Option<egui::IconData> {
    const ICON_BYTES: &[u8] = include_bytes!("../../../assets/icon.ico");
    // Decodificar a resolução 256×256 do .ico
    let img = image::load_from_memory(ICON_BYTES).ok()?;
    let img = img.into_rgba8();
    let (w, h) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width: w,
        height: h,
    })
}

/// Configura fontes para suporte a caracteres pt-BR.
fn setup_fonts(ctx: &egui::Context) {
    let fonts = egui::FontDefinitions::default();
    // Noto Sans (OFL-1.1) seria adicionada aqui quando disponível nos assets/fonts/.
    // Por ora, usar fonte padrão do egui que suporta ASCII + Latin-1.
    ctx.set_fonts(fonts);
}

/// Tema escuro sóbrio para ambiente institucional.
fn dark_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    // Leve suavização das bordas
    v.window_corner_radius = egui::CornerRadius::same(4);
    v.menu_corner_radius = egui::CornerRadius::same(4);
    v.widgets.noninteractive.corner_radius = egui::CornerRadius::same(3);
    v.widgets.inactive.corner_radius = egui::CornerRadius::same(3);
    v.widgets.hovered.corner_radius = egui::CornerRadius::same(3);
    v.widgets.active.corner_radius = egui::CornerRadius::same(3);
    // Cor de acento institucional (azul MPTO)
    v.selection.bg_fill = egui::Color32::from_rgb(0, 90, 160);
    v
}

/// Define o AppUserModelID para agrupamento correto na barra de tarefas do Windows.
#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
fn set_app_user_model_id() {
    use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
    use windows::core::HSTRING;

    let app_id = HSTRING::from("br.mp.to.gaeco.clio");
    // SAFETY: chamada à Windows API documentada; veja módulo wake_lock para justificativa.
    if let Err(e) = unsafe { SetCurrentProcessExplicitAppUserModelID(&app_id) } {
        tracing::warn!("AppUserModelID não definido: {e}");
    } else {
        info!("AppUserModelID definido: br.mp.to.gaeco.clio");
    }
}
