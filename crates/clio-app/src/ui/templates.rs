//! Aba de editor de modelos de texto.

use crate::app_state::{AppState, TemplateTab};
use clio_report::TemplateEngine;
use egui::Ui;

pub fn show_templates_tab(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Editor de Modelos de Texto");
    ui.add_space(4.0);

    // Abas Extrato / Relatório
    ui.horizontal(|ui| {
        if ui
            .selectable_label(
                state.template_editor.active_tab == TemplateTab::Extrato,
                "Extrato",
            )
            .clicked()
        {
            state.template_editor.active_tab = TemplateTab::Extrato;
        }
        if ui
            .selectable_label(
                state.template_editor.active_tab == TemplateTab::Relatorio,
                "Relatório",
            )
            .clicked()
        {
            state.template_editor.active_tab = TemplateTab::Relatorio;
        }
    });
    ui.separator();

    let engine = TemplateEngine::new();
    let ctx_exemplo = clio_report::ReportContext::exemplo();

    match state.template_editor.active_tab {
        TemplateTab::Extrato => {
            // Inicializar com padrão se vazio
            if state.template_editor.extrato_source.is_empty() {
                state.template_editor.extrato_source = state
                    .config
                    .modelos
                    .extrato_template
                    .clone()
                    .unwrap_or_else(|| clio_report::template::DEFAULT_EXTRATO.to_string());
            }

            template_editor_pane(
                ui,
                "extrato_editor",
                &mut state.template_editor.extrato_source,
                &mut state.template_editor.extrato_preview,
                &mut state.template_editor.extrato_error,
                &engine,
                &ctx_exemplo,
            );

            ui.horizontal(|ui| {
                if ui.button("💾 Salvar modelo").clicked() {
                    match engine.validar(&state.template_editor.extrato_source) {
                        Ok(_) => {
                            state
                                .config
                                .modelos
                                .salvar_extrato(state.template_editor.extrato_source.clone());
                            state.config_dirty = true;
                            state.push_toast(crate::app_state::Toast::success(
                                "Modelo do Extrato salvo",
                            ));
                        }
                        Err(e) => {
                            state.push_toast(crate::app_state::Toast::error(format!(
                                "Modelo inválido: {e}"
                            )));
                        }
                    }
                }
                if ui.button("🔄 Restaurar padrão").clicked() {
                    state.template_editor.extrato_source =
                        clio_report::template::DEFAULT_EXTRATO.to_string();
                    state.config.modelos.extrato_template = None;
                    state.config_dirty = true;
                }
                if !state.config.modelos.extrato_historico.is_empty() {
                    ui.menu_button("📜 Histórico", |ui| {
                        for (i, hist) in state
                            .config
                            .modelos
                            .extrato_historico
                            .iter()
                            .enumerate()
                            .rev()
                        {
                            if ui.button(format!("Versão {}", i + 1)).clicked() {
                                state.template_editor.extrato_source = hist.clone();
                                ui.close_menu();
                            }
                        }
                    });
                }
            });
        }
        TemplateTab::Relatorio => {
            if state.template_editor.relatorio_source.is_empty() {
                state.template_editor.relatorio_source = state
                    .config
                    .modelos
                    .relatorio_template
                    .clone()
                    .unwrap_or_else(|| clio_report::template::DEFAULT_RELATORIO.to_string());
            }

            template_editor_pane(
                ui,
                "relatorio_editor",
                &mut state.template_editor.relatorio_source,
                &mut state.template_editor.relatorio_preview,
                &mut state.template_editor.relatorio_error,
                &engine,
                &ctx_exemplo,
            );

            ui.horizontal(|ui| {
                if ui.button("💾 Salvar modelo").clicked() {
                    match engine.validar(&state.template_editor.relatorio_source) {
                        Ok(_) => {
                            state
                                .config
                                .modelos
                                .salvar_relatorio(state.template_editor.relatorio_source.clone());
                            state.config_dirty = true;
                            state.push_toast(crate::app_state::Toast::success(
                                "Modelo do Relatório salvo",
                            ));
                        }
                        Err(e) => {
                            state.push_toast(crate::app_state::Toast::error(format!(
                                "Modelo inválido: {e}"
                            )));
                        }
                    }
                }
                if ui.button("🔄 Restaurar padrão").clicked() {
                    state.template_editor.relatorio_source =
                        clio_report::template::DEFAULT_RELATORIO.to_string();
                    state.config.modelos.relatorio_template = None;
                    state.config_dirty = true;
                }
            });
        }
    }
}

fn template_editor_pane(
    ui: &mut Ui,
    id: &str,
    source: &mut String,
    preview: &mut String,
    error: &mut Option<String>,
    engine: &TemplateEngine,
    ctx: &clio_report::ReportContext,
) {
    let avail_w = ui.available_width();
    let half_w = avail_w / 2.0 - 8.0;

    ui.horizontal_top(|ui| {
        // Painel de edição
        ui.vertical(|ui| {
            ui.set_width(half_w);
            ui.label(egui::RichText::new("Modelo (Jinja2)").strong());
            let resp = ui.add(
                egui::TextEdit::multiline(source)
                    .font(egui::FontId::monospace(12.0))
                    .desired_width(half_w)
                    .desired_rows(20),
            );
            if resp.changed() {
                // Atualizar prévia ao vivo
                match engine.renderizar(source, ctx) {
                    Ok(rendered) => {
                        *preview = rendered;
                        *error = None;
                    }
                    Err(e) => {
                        *error = Some(e.to_string());
                    }
                }
            }
            if let Some(err) = error {
                ui.add_space(4.0);
                ui.colored_label(
                    egui::Color32::from_rgb(255, 100, 100),
                    egui::RichText::new(format!("⚠ {err}")).size(12.0),
                );
            }
        });

        ui.separator();

        // Painel de prévia
        ui.vertical(|ui| {
            ui.set_width(half_w);
            ui.label(egui::RichText::new("Prévia (dados de exemplo)").strong());
            if preview.is_empty() {
                // Renderizar uma vez ao abrir
                if let Ok(r) = engine.renderizar(source, ctx) {
                    *preview = r;
                }
            }
            egui::ScrollArea::vertical()
                .id_salt(format!("{id}_preview_scroll"))
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(preview)
                            .font(egui::FontId::proportional(11.0))
                            .desired_width(half_w)
                            .interactive(false),
                    );
                });
        });
    });
}
