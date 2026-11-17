//! Diálogos modais: confirmação, recuperação, orientações legais.

use crate::app_state::{AppState, ConfirmAction};
use egui::Context;
use tracing::info;

pub fn show_confirm_dialog(ctx: &Context, state: &mut AppState) {
    let Some(dialog) = &state.confirm_dialog else {
        return;
    };
    let title = dialog.title.clone();

    egui::Window::new(&title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .min_width(380.0)
        .show(ctx, |ui| {
            let dialog = state.confirm_dialog.as_mut().unwrap();
            ui.add_space(8.0);
            ui.label(&dialog.message);
            ui.add_space(8.0);

            if let Some(required) = &dialog.require_text.clone() {
                ui.label(format!("Para confirmar, digite: \"{required}\""));
                ui.text_edit_singleline(&mut dialog.typed_text);
                ui.add_space(4.0);
            }

            ui.horizontal(|ui| {
                let can_confirm = state.confirm_dialog.as_ref().unwrap().can_confirm();
                let action = state.confirm_dialog.as_ref().unwrap().action.clone();

                if ui
                    .add_enabled(
                        can_confirm,
                        egui::Button::new(
                            egui::RichText::new("Confirmar")
                                .color(egui::Color32::from_rgb(255, 100, 100)),
                        )
                        .min_size(egui::vec2(120.0, 32.0)),
                    )
                    .clicked()
                {
                    state.confirm_dialog = None;
                    execute_confirm_action(state, action);
                }

                ui.add_space(8.0);

                if ui
                    .add(egui::Button::new("Cancelar").min_size(egui::vec2(100.0, 32.0)))
                    .clicked()
                {
                    state.confirm_dialog = None;
                }
            });
            ui.add_space(8.0);
        });
}

fn execute_confirm_action(state: &mut AppState, action: ConfirmAction) {
    match action {
        ConfirmAction::StopRecording => {
            if let Some(rec) = &state.active_recording {
                let _ = rec.pipeline.send(clio_media::PipelineCommand::Stop);
                info!("Gravação finalizada pelo usuário");
            }
            state.active_recording = None;
            state.push_toast(crate::app_state::Toast::success(
                "Gravação finalizada e salva na biblioteca",
            ));
        }
        ConfirmAction::DeleteRecording { id, path } => {
            let _ = std::fs::remove_file(&path);
            let _ = state.db.remover_gravacao(&id);
            info!("Gravação removida: {id}");
            state.push_toast(crate::app_state::Toast::info("Gravação excluída"));
        }
        ConfirmAction::CloseApp => {
            if let Some(rec) = &state.active_recording {
                let _ = rec.pipeline.send(clio_media::PipelineCommand::Stop);
            }
            state.active_recording = None;
            state.save_config_if_dirty();
            std::process::exit(0);
        }
        ConfirmAction::Custom(_) => {}
    }
}

pub fn show_legal_modal(ctx: &Context, state: &mut AppState) {
    egui::Window::new("⚖ Orientações sobre o Sistema")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .min_width(500.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                ui.heading("Clio — Gravador de Oitivas e Audiências");
                ui.add_space(8.0);
                ui.label(
"Este sistema grava, organiza e documenta oitivas e audiências no âmbito do sistema de justiça.

O que o Clio faz:
• Registra vídeo e áudio com tarja probatória gravada na imagem (burn-in).
• Calcula e verifica o hash SHA-256 do arquivo de vídeo ao encerrar a gravação.
• Gera Extrato (certidão técnica de integridade) e Relatório circunstanciado.
• Oferece um selo local Ed25519 para detectar adulteração do manifesto.

O que o Clio NÃO faz:
• Não garante validade jurídica plena por si só — a eficácia probatória cabe à autoridade competente.
• Não substitui assinatura digital qualificada (ICP-Brasil) nem carimbo do tempo de terceiro.
• Não pode impedir adulteração por operador com acesso físico ao computador.
• Não faz conexões de rede nem telemetria.

O uso adequado do sistema, o controle de acesso ao computador e a guarda da mídia são responsabilidade da unidade.

Para validade reforçada, recomenda-se a assinatura digital ICP-Brasil e o uso de carimbo do tempo (TSA)."
                );
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(egui::RichText::new("Entendi — Prosseguir").strong())
                    .clicked()
                {
                    state.show_legal_modal = false;
                }
            });
        });
}

pub fn show_recovery_dialog(ctx: &Context, state: &mut AppState) {
    let n = state.sessoes_pendentes.len();
    egui::Window::new(format!("⚠ {n} sessão(ões) sem encerramento regular"))
        .collapsible(false)
        .resizable(true)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .min_width(480.0)
        .show(ctx, |ui| {
            ui.label(
                "As sessões abaixo não tiveram encerramento regular (possível queda do sistema).\
                 \nEscolha Resgatar para recuperar o vídeo ou Manter para análise posterior.",
            );
            ui.add_space(8.0);

            let mut to_rescue: Option<usize> = None;
            let mut to_keep: Option<usize> = None;

            for (i, sessao) in state.sessoes_pendentes.iter().enumerate() {
                ui.group(|ui| {
                    ui.label(egui::RichText::new(&sessao.depoente_nome).strong());
                    ui.label(format!("Autos: {}", sessao.processo_numero));
                    ui.label(format!("Início: {}", sessao.inicio_iso8601));
                    ui.label(format!("Arquivo: {}", sessao.arquivo_path));
                    ui.horizontal(|ui| {
                        if ui.button("🔧 Resgatar").clicked() {
                            to_rescue = Some(i);
                        }
                        if ui.button("📁 Manter para análise").clicked() {
                            to_keep = Some(i);
                        }
                    });
                });
                ui.add_space(4.0);
            }

            if let Some(i) = to_rescue {
                let sessao = state.sessoes_pendentes.remove(i);
                info!("Resgatando sessão: {}", sessao.id);
                // Marcar como recuperada no banco
                let journal = clio_store::SessionJournal::for_session(&state.db, &sessao.id);
                let _ = journal.marcar_recuperada();
                state.push_toast(crate::app_state::Toast::info(format!(
                    "Sessão de '{}' resgatada",
                    sessao.depoente_nome
                )));
            }
            if let Some(i) = to_keep {
                state.sessoes_pendentes.remove(i);
            }
        });
}
