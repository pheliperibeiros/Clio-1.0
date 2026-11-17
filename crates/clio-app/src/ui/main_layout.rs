//! Layout principal: barra de abas + conteúdo.

use super::{library, recording, settings, templates};
use crate::app_state::{ActiveTab, AppState};
use eframe::Frame;
use egui::Context;

pub fn show(ctx: &Context, _frame: &mut Frame, state: &mut AppState) {
    // ── Barra de abas superior ─────────────────────────────────────────────
    egui::TopBottomPanel::top("tab_bar")
        .min_height(44.0)
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(8.0);

                // Logomarca + nome
                ui.label(egui::RichText::new("⚖ Clio").size(18.0).strong());
                ui.separator();

                for (tab, label, _shortcut) in [
                    (ActiveTab::Gravar, "🎥  Gravar", ""),
                    (ActiveTab::Biblioteca, "📂  Biblioteca", ""),
                    (ActiveTab::Modelos, "📄  Modelos", ""),
                    (ActiveTab::Configuracoes, "⚙  Configurações", ""),
                ] {
                    let selected = state.tab == tab;
                    let btn = egui::Button::new(egui::RichText::new(label).size(14.0))
                        .selected(selected)
                        .min_size(egui::vec2(110.0, 36.0));

                    if ui.add(btn).clicked() {
                        state.tab = tab;
                    }
                }

                // Indicador de gravação ativa na barra
                if let Some(rec) = &state.active_recording {
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(8.0);
                    let cronometro = format_secs(rec.secs_recorded);
                    let cor = if rec.secs_recorded % 2 == 0 {
                        egui::Color32::from_rgb(220, 40, 40)
                    } else {
                        egui::Color32::from_rgb(180, 20, 20)
                    };
                    ui.label(
                        egui::RichText::new(format!("● REC  {cronometro}"))
                            .color(cor)
                            .size(15.0)
                            .strong(),
                    );
                }

                // Aviso de configuração corrompida/restaurada
                if !state.config_warnings.is_empty() {
                    ui.add_space(8.0);
                    ui.colored_label(egui::Color32::YELLOW, "⚠ Configuração restaurada")
                        .on_hover_text(state.config_warnings.join("\n"));
                }
            });
        });

    // ── Conteúdo da aba ────────────────────────────────────────────────────
    egui::CentralPanel::default().show(ctx, |ui| match state.tab {
        ActiveTab::Gravar => recording::show_recording_tab(ui, state),
        ActiveTab::Biblioteca => library::show_library_tab(ui, state),
        ActiveTab::Modelos => templates::show_templates_tab(ui, state),
        ActiveTab::Configuracoes => settings::show_settings_tab(ui, state),
    });
}

pub fn format_secs(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}
