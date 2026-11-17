//! Raiz da aplicação eframe — orquestra o loop de UI.

use eframe::Frame;
use egui::Context;
use crate::app_state::AppState;
use crate::ui;

pub struct ClioApp {
    state: AppState,
}

impl ClioApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, state: AppState) -> Self {
        Self { state }
    }
}

impl eframe::App for ClioApp {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        // Processar eventos do pipeline (se gravação ativa)
        ui::recording::process_pipeline_events(&mut self.state);

        // Processar eventos de dispositivos
        ui::recording::process_device_events(&mut self.state);

        // Fechar com gravação ativa → confirmação
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.state.active_recording.is_some() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.state.confirm_dialog = Some(crate::app_state::ConfirmDialog {
                    title: "Fechar com gravação ativa".to_string(),
                    message: "Há uma gravação em andamento. Deseja finalizar e fechar?".to_string(),
                    require_text: None,
                    typed_text: String::new(),
                    action: crate::app_state::ConfirmAction::CloseApp,
                });
            } else {
                // Salvar configuração ao fechar
                self.state.save_config_if_dirty();
                // Salvar geometria da janela
                if let Some(rect) = ctx.input(|i| i.viewport().inner_rect) {
                    self.state.config.janela.pos_x = Some(rect.left());
                    self.state.config.janela.pos_y = Some(rect.top());
                    self.state.config.janela.width = Some(rect.width());
                    self.state.config.janela.height = Some(rect.height());
                    self.state.config.janela.maximized =
                        ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                    let _ = self.state.config_storage.salvar(&self.state.config);
                }
            }
        }

        // Tick dos toasts
        self.state.tick_toasts();

        // ── Dialogo de confirmação (modal, bloqueante) ─────────────────────────
        if self.state.confirm_dialog.is_some() {
            ui::dialogs::show_confirm_dialog(ctx, &mut self.state);
            return; // Bloquear o resto da UI enquanto o diálogo estiver aberto
        }

        // ── Assistente de primeira execução ────────────────────────────────────
        if self.state.show_first_run_wizard {
            ui::wizard::show_first_run_wizard(ctx, &mut self.state);
            return;
        }

        // ── Modal de orientações legais ────────────────────────────────────────
        if self.state.show_legal_modal {
            ui::dialogs::show_legal_modal(ctx, &mut self.state);
        }

        // ── Recuperação de sessões pendentes ───────────────────────────────────
        if !self.state.sessoes_pendentes.is_empty() {
            ui::dialogs::show_recovery_dialog(ctx, &mut self.state);
            return;
        }

        // ── Layout principal ───────────────────────────────────────────────────
        ui::main_layout::show(ctx, frame, &mut self.state);

        // Toasts (sobrepostos)
        ui::toasts::show_toasts(ctx, &mut self.state);

        // Solicitar novo frame para animações (cronômetro, VU)
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}
