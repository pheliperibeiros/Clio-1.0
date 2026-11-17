//! Estado global do aplicativo — compartilhado entre frames da UI.

use parking_lot::Mutex;
use std::sync::Arc;
use clio_config::{AppConfig, ConfigStorage};
use clio_core::models::{Depoente, Processo, Defesa, Participante};
use clio_core::recording::RecordingState;
use clio_core::types::Timestamp;
use clio_media::{PipelineHandle, DeviceMonitor, VuLevel};
use clio_store::{Database, session_journal::SessaoPendente};

/// Aba ativa na UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Gravar,
    Biblioteca,
    Modelos,
    Configuracoes,
}

impl Default for ActiveTab {
    fn default() -> Self { Self::Gravar }
}

/// Estado de uma gravação em andamento.
#[derive(Debug)]
pub struct ActiveRecording {
    pub session_id: String,
    pub state: RecordingState,
    pub started_at: Timestamp,
    pub output_path: std::path::PathBuf,
    pub secs_recorded: u64,
    pub secs_paused: u64,
    pub pipeline: PipelineHandle,
    /// Último frame de prévia disponível (RGBA, 960×540).
    pub preview_frame: Option<(u32, u32, Vec<u8>)>,
    pub vu_mic1: VuLevel,
    pub vu_mic2: VuLevel,
    pub mic2_active: bool,
    pub mic1_muted: bool,
    pub mic2_muted: bool,
    pub layout: clio_config::schema::Layout,
    pub disk_minutes_remaining: f64,
    pub disk_bytes_available: u64,
    /// Avisos ativos (mostrados como banners).
    pub warnings: Vec<String>,
}

/// Estado da sessão atual (formulário de identificação).
#[derive(Debug, Default, Clone)]
pub struct SessionDraft {
    pub depoente: Depoente,
    pub processo: Processo,
    pub participantes: Vec<Participante>,
    pub defesas: Vec<Defesa>,
    /// Validação inline: mensagens por campo.
    pub validation_msgs: std::collections::HashMap<String, String>,
}

impl SessionDraft {
    pub fn is_valid_to_start(&self) -> bool {
        !self.depoente.nome.trim().is_empty()
            && !self.processo.numero.trim().is_empty()
    }

    pub fn blocking_issues(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if self.depoente.nome.trim().is_empty() {
            issues.push("Nome do depoente é obrigatório".to_string());
        }
        if self.processo.numero.trim().is_empty() {
            issues.push("Número dos autos é obrigatório".to_string());
        }
        issues
    }
}

/// Estado completo do aplicativo.
pub struct AppState {
    pub config: AppConfig,
    pub config_storage: ConfigStorage,
    pub config_dirty: bool,
    pub config_warnings: Vec<String>,
    /// Sessões sem encerramento regular (recuperação).
    pub sessoes_pendentes: Vec<SessaoPendente>,
    pub db: Database,
    pub tab: ActiveTab,
    pub session_draft: SessionDraft,
    pub active_recording: Option<ActiveRecording>,
    pub device_monitor: DeviceMonitor,
    /// Toasts / notificações temporárias.
    pub toasts: Vec<Toast>,
    /// Dialogo de confirmação aberto (ação destrutiva).
    pub confirm_dialog: Option<ConfirmDialog>,
    /// Assistente de primeira execução.
    pub show_first_run_wizard: bool,
    /// Modal "Orientações legais".
    pub show_legal_modal: bool,
    /// Busca na biblioteca.
    pub library_search: String,
    /// Editor de modelos.
    pub template_editor: TemplateEditorState,
    /// Aviso de transparência da tarja.
    pub tarja_aviso_transparencia: bool,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        config_storage: ConfigStorage,
        config_warnings: Vec<String>,
        sessoes_pendentes: Vec<SessaoPendente>,
        db: Database,
    ) -> Self {
        let show_first_run = !config.preferencias.primeiro_uso_concluido;
        let device_monitor = DeviceMonitor::new();
        device_monitor.start().ok();

        Self {
            config,
            config_storage,
            config_dirty: false,
            config_warnings,
            sessoes_pendentes,
            db,
            tab: ActiveTab::default(),
            session_draft: SessionDraft::default(),
            active_recording: None,
            device_monitor,
            toasts: Vec::new(),
            confirm_dialog: None,
            show_first_run_wizard: show_first_run,
            show_legal_modal: show_first_run,
            library_search: String::new(),
            template_editor: TemplateEditorState::default(),
            tarja_aviso_transparencia: false,
        }
    }

    /// Salva a configuração se houver mudanças pendentes.
    pub fn save_config_if_dirty(&mut self) {
        if self.config_dirty {
            if let Err(e) = self.config_storage.salvar(&self.config) {
                self.push_toast(Toast::error(format!("Erro ao salvar configuração: {e}")));
            } else {
                self.config_dirty = false;
            }
        }
    }

    pub fn push_toast(&mut self, toast: Toast) {
        self.toasts.push(toast);
    }

    pub fn tick_toasts(&mut self) {
        self.toasts.retain(|t| !t.expired());
    }
}

/// Notificação temporária.
#[derive(Debug)]
pub struct Toast {
    pub message: String,
    pub kind: ToastKind,
    created: std::time::Instant,
    duration_secs: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

impl Toast {
    pub fn info(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), kind: ToastKind::Info, created: std::time::Instant::now(), duration_secs: 4.0 }
    }
    pub fn success(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), kind: ToastKind::Success, created: std::time::Instant::now(), duration_secs: 4.0 }
    }
    pub fn warning(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), kind: ToastKind::Warning, created: std::time::Instant::now(), duration_secs: 6.0 }
    }
    pub fn error(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), kind: ToastKind::Error, created: std::time::Instant::now(), duration_secs: 8.0 }
    }
    pub fn expired(&self) -> bool {
        self.created.elapsed().as_secs_f32() > self.duration_secs
    }
    pub fn color(&self) -> egui::Color32 {
        match self.kind {
            ToastKind::Info    => egui::Color32::from_rgb(50, 120, 200),
            ToastKind::Success => egui::Color32::from_rgb(40, 160, 80),
            ToastKind::Warning => egui::Color32::from_rgb(200, 150, 20),
            ToastKind::Error   => egui::Color32::from_rgb(200, 50, 50),
        }
    }
}

/// Diálogo de confirmação para ações destrutivas.
#[derive(Debug, Clone)]
pub struct ConfirmDialog {
    pub title: String,
    pub message: String,
    /// Para exclusão: exige que o usuário digite este texto.
    pub require_text: Option<String>,
    pub typed_text: String,
    pub action: ConfirmAction,
}

#[derive(Debug, Clone)]
pub enum ConfirmAction {
    StopRecording,
    DeleteRecording { id: String, path: String },
    CloseApp,
    Custom(String),
}

impl ConfirmDialog {
    pub fn can_confirm(&self) -> bool {
        match &self.require_text {
            Some(required) => self.typed_text.trim() == required.trim(),
            None => true,
        }
    }
}

/// Estado do editor de modelos de texto.
#[derive(Debug, Default)]
pub struct TemplateEditorState {
    pub extrato_source: String,
    pub relatorio_source: String,
    pub extrato_preview: String,
    pub relatorio_preview: String,
    pub extrato_error: Option<String>,
    pub relatorio_error: Option<String>,
    pub active_tab: TemplateTab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TemplateTab {
    #[default]
    Extrato,
    Relatorio,
}
