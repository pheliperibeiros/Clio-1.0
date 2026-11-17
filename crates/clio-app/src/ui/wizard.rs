//! Assistente de primeira execução.

use egui::Context;
use crate::app_state::AppState;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
enum WizardStep {
    #[default]
    Institucional,
    Pasta,
    TesteDispositivos,
    Conclusao,
}

pub fn show_first_run_wizard(ctx: &Context, state: &mut AppState) {
    egui::Window::new("Bem-vindo ao Clio")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .min_width(500.0)
        .show(ctx, |ui| {
            ui.heading("Configuração inicial");
            ui.add_space(8.0);
            ui.label("Preencha as informações básicas para começar a usar o Clio.");
            ui.separator();

            // ── Passo 1: Instituição ───────────────────────────────────────
            ui.label(egui::RichText::new("1. Instituição e autoridade").strong());
            ui.label("Órgão:");
            ui.text_edit_singleline(&mut state.config.instituicao.orgao);
            ui.label("Título da autoridade:");
            ui.text_edit_singleline(&mut state.config.instituicao.autoridade_titulo);
            ui.label("Nome da autoridade:");
            ui.text_edit_singleline(&mut state.config.instituicao.autoridade_nome);
            ui.label("Responsável pela gravação:");
            ui.text_edit_singleline(&mut state.config.instituicao.responsavel_gravacao);
            ui.label("Cidade de emissão dos documentos:");
            ui.text_edit_singleline(&mut state.config.emissao.local);

            ui.separator();
            // ── Passo 2: Pasta de gravações ────────────────────────────────
            ui.label(egui::RichText::new("2. Pasta de gravações").strong());
            let pasta_str = state.config.gravacao.pasta_destino
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "Padrão (Vídeos/Clio)".to_string());
            ui.label(format!("Pasta atual: {pasta_str}"));
            if ui.button("Selecionar pasta…").clicked() {
                // Integração com rfd (dialog de seleção de pasta) na fase final
                state.push_toast(crate::app_state::Toast::info(
                    "Seleção de pasta será integrada na fase (g)"
                ));
            }

            ui.separator();
            // ── Passo 3: Dispositivos ──────────────────────────────────────
            ui.label(egui::RichText::new("3. Verificação de dispositivos").strong());
            let cameras = state.device_monitor.cameras();
            let mics = state.device_monitor.microphones();
            ui.label(format!("Câmeras detectadas: {}", cameras.len()));
            ui.label(format!("Microfones detectados: {}", mics.len()));
            if cameras.is_empty() {
                ui.colored_label(egui::Color32::YELLOW, "⚠ Nenhuma câmera detectada. Conecte uma câmera e reinicie.");
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.add(
                    egui::Button::new(egui::RichText::new("Concluir configuração").strong())
                        .min_size(egui::vec2(200.0, 36.0))
                ).clicked() {
                    state.config.preferencias.primeiro_uso_concluido = true;
                    state.config_dirty = true;
                    state.save_config_if_dirty();
                    state.show_first_run_wizard = false;
                    state.show_legal_modal = true;
                }
            });
        });
}
