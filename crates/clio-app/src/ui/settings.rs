//! Aba de Configurações.

use crate::app_state::AppState;
use egui::Ui;

pub fn show_settings_tab(ui: &mut Ui, state: &mut AppState) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Configurações");
        ui.add_space(8.0);

        // ── Instituição ────────────────────────────────────────────────────
        ui.collapsing("🏛 Instituição e Autoridade", |ui| {
            let inst = &mut state.config.instituicao;
            let mut dirty = false;
            ui.label("Órgão:");
            dirty |= ui.text_edit_singleline(&mut inst.orgao).changed();
            ui.label("Título da autoridade:");
            dirty |= ui
                .text_edit_singleline(&mut inst.autoridade_titulo)
                .changed();
            ui.label("Nome da autoridade:");
            dirty |= ui.text_edit_singleline(&mut inst.autoridade_nome).changed();
            ui.label("Responsável pela gravação:");
            dirty |= ui
                .text_edit_singleline(&mut inst.responsavel_gravacao)
                .changed();
            ui.label("Cidade de emissão:");
            dirty |= ui
                .text_edit_singleline(&mut state.config.emissao.local)
                .changed();
            if dirty {
                state.config_dirty = true;
            }
        });

        ui.add_space(4.0);

        // ── Gravação ───────────────────────────────────────────────────────
        ui.collapsing("🎥 Gravação", |ui| {
            let grav = &mut state.config.gravacao;
            let mut dirty = false;

            ui.horizontal(|ui| {
                ui.label("Bitrate de vídeo (kbps):");
                dirty |= ui
                    .add(
                        egui::DragValue::new(&mut grav.video_bitrate_kbps)
                            .range(500..=50000)
                            .speed(100.0),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Bitrate de áudio (kbps):");
                dirty |= ui
                    .add(
                        egui::DragValue::new(&mut grav.audio_bitrate_kbps)
                            .range(64..=320)
                            .speed(8.0),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Alertar com menos de (min):");
                dirty |= ui
                    .add(egui::DragValue::new(&mut grav.alerta_espaco_min_minutos).range(5..=120))
                    .changed();
            });

            if dirty {
                state.config_dirty = true;
            }

            ui.label("Pasta de destino:");
            let pasta = grav
                .pasta_destino
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "Padrão".to_string());
            ui.label(&pasta);
            if ui.button("Selecionar…").clicked() {
                state.push_toast(crate::app_state::Toast::info(
                    "Seleção de pasta disponível na fase (g)",
                ));
            }
        });

        ui.add_space(4.0);

        // ── Áudio ──────────────────────────────────────────────────────────
        ui.collapsing("🎙 Áudio", |ui| {
            let audio = &mut state.config.audio;
            let mut dirty = false;

            ui.label("Modo de gravação:");
            ui.horizontal(|ui| {
                for modo in [
                    clio_config::schema::ModoAudio::FaixasSeparadas,
                    clio_config::schema::ModoAudio::MixagemUnica,
                ] {
                    if ui
                        .selectable_label(audio.modo == modo, modo.label())
                        .clicked()
                    {
                        audio.modo = modo;
                        dirty = true;
                    }
                }
            });

            ui.label("Rótulo Microfone 1:");
            dirty |= ui.text_edit_singleline(&mut audio.mic1_rotulo).changed();
            ui.label("Rótulo Microfone 2:");
            dirty |= ui.text_edit_singleline(&mut audio.mic2_rotulo).changed();
            dirty |= ui
                .checkbox(
                    &mut audio.mic2_ativo_por_padrao,
                    "Microfone 2 ativo por padrão",
                )
                .changed();

            if dirty {
                state.config_dirty = true;
            }
        });

        ui.add_space(4.0);

        // ── Identidade visual ──────────────────────────────────────────────
        ui.collapsing("🎨 Identidade Visual", |ui| {
            ui.label("Cabeçalho dos documentos:");
            let header_label = state
                .config
                .identidade_visual
                .cabecalho_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "Padrão (header.png embutido)".to_string());
            ui.label(&header_label);
            ui.horizontal(|ui| {
                if ui.button("Substituir…").clicked() {
                    state.push_toast(crate::app_state::Toast::info(
                        "Seleção de imagem disponível na fase (f)",
                    ));
                }
                if ui.button("Restaurar padrão").clicked() {
                    state.config.identidade_visual.cabecalho_path = None;
                    state.config_dirty = true;
                }
            });

            ui.add_space(4.0);
            ui.label(
                "ℹ O Clio não gera arquivo .dot (binário legado Word 97-2003). \
                       O .dotx abre no Word 2007 ou superior; para .dot, abra o .dotx \
                       no Word e use 'Salvar como'.",
            );
        });

        ui.add_space(4.0);

        // ── Importar / Exportar config ─────────────────────────────────────
        ui.collapsing("📤 Importar / Exportar configurações", |ui| {
            ui.horizontal(|ui| {
                if ui.button("Exportar config.toml…").clicked() {
                    state.push_toast(crate::app_state::Toast::info(
                        "Exportação disponível na fase (g)",
                    ));
                }
                if ui.button("Importar config.toml…").clicked() {
                    state.push_toast(crate::app_state::Toast::info(
                        "Importação disponível na fase (g)",
                    ));
                }
            });
        });

        ui.add_space(4.0);

        // ── Sobre ──────────────────────────────────────────────────────────
        ui.collapsing("ℹ Sobre o Clio", |ui| {
            ui.label(format!("Clio versão {}", crate::APP_VERSION));
            ui.label(format!("Build: {}", crate::APP_BUILD));
            ui.label("GAECO / Ministério Público do Estado do Tocantins");
            ui.separator();
            ui.label(egui::RichText::new("Limitações conhecidas:").strong());
            ui.label("• Sem assinatura ICP-Brasil nem carimbo do tempo externo.");
            ui.label("• O selo Ed25519 é local — não equivale a certificado digital.");
            ui.label("• Arquivo-mestre em MKV; MP4 fragmentado como cópia derivada opcional.");
            ui.label("• Sem geração de .dot binário (formato legado Word 97).");
            ui.label("• A validade probatória cabe à autoridade competente.");
        });

        // Botão salvar flutuante
        ui.add_space(16.0);
        if state.config_dirty {
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("💾 Salvar configurações").strong())
                        .min_size(egui::vec2(200.0, 36.0)),
                )
                .clicked()
            {
                state.save_config_if_dirty();
                state.push_toast(crate::app_state::Toast::success("Configurações salvas"));
            }
        }
    });
}
