//! Aba Biblioteca de gravações.

use crate::app_state::{AppState, ConfirmAction, ConfirmDialog};
use egui::Ui;

pub fn show_library_tab(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Biblioteca de Gravações");
    ui.add_space(4.0);

    // Busca
    ui.horizontal(|ui| {
        ui.label("Buscar:");
        ui.text_edit_singleline(&mut state.library_search);
        if ui.small_button("✕").clicked() {
            state.library_search.clear();
        }
    });
    ui.separator();

    let filtro = if state.library_search.is_empty() {
        None
    } else {
        Some(state.library_search.as_str())
    };

    let gravacoes = state.db.listar_gravacoes(filtro).unwrap_or_default();

    if gravacoes.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new("Nenhuma gravação encontrada")
                    .color(egui::Color32::GRAY)
                    .size(16.0),
            );
        });
        return;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        for grav in &gravacoes {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    // Status badge
                    let (badge_cor, badge_txt) = match grav.status {
                        clio_store::StatusGravacao::Concluida  => (egui::Color32::GREEN,  "Concluída"),
                        clio_store::StatusGravacao::Recuperada => (egui::Color32::YELLOW, "Recuperada"),
                        clio_store::StatusGravacao::Exportada  => (egui::Color32::LIGHT_BLUE, "Exportada"),
                    };
                    ui.colored_label(badge_cor, badge_txt);
                    ui.separator();
                    ui.label(egui::RichText::new(&grav.depoente_nome).strong());
                    ui.label(format!("— {}", grav.depoente_qualificacao));
                });

                ui.horizontal(|ui| {
                    ui.label(format!("Autos: {}", grav.processo_numero));
                    ui.separator();
                    ui.label(format!("Data: {}", &grav.inicio_iso8601[..10]));
                    if let Some(dur) = grav.duracao_liquida_secs {
                        ui.separator();
                        ui.label(format!("Duração: {}", format_duration(dur)));
                    }
                    if let Some(size) = grav.arquivo_tamanho_bytes {
                        ui.separator();
                        ui.label(format!("Tamanho: {}", format_bytes(size)));
                    }
                });

                if let Some(hash) = &grav.arquivo_sha256 {
                    ui.label(
                        egui::RichText::new(format!("SHA-256: {}…", &hash[..16.min(hash.len())]))
                            .monospace()
                            .size(11.0)
                            .color(egui::Color32::from_gray(160)),
                    );
                }

                ui.horizontal(|ui| {
                    if ui.small_button("▶ Abrir vídeo").clicked() {
                        open_in_system(&grav.arquivo_path);
                    }
                    if ui.small_button("📄 Extrato PDF").clicked() {
                        // Integração de exportação na fase (e)
                        state.push_toast(crate::app_state::Toast::info("Exportação PDF implementada na fase (e)"));
                    }
                    if ui.small_button("📝 Relatório DOCX").clicked() {
                        state.push_toast(crate::app_state::Toast::info("Exportação DOCX implementada na fase (e)"));
                    }
                    if ui.small_button("🔍 Verificar integridade").clicked() {
                        state.push_toast(crate::app_state::Toast::info("Verificação implementada na fase (d)"));
                    }
                    if ui.small_button("📦 Manifesto").clicked() {
                        if let Some(mp) = &grav.manifesto_path {
                            open_in_system(mp);
                        }
                    }

                    // Excluir — confirmação dupla com digitação do nome
                    if ui.add(
                        egui::Button::new(egui::RichText::new("🗑 Excluir").color(egui::Color32::from_rgb(220, 60, 60)))
                    ).clicked() {
                        state.confirm_dialog = Some(ConfirmDialog {
                            title: "Excluir gravação".to_string(),
                            message: format!(
                                "Esta ação é irreversível. O arquivo de vídeo será apagado permanentemente.\n\
                                 Para confirmar, digite o nome do depoente: \"{}\"",
                                grav.depoente_nome
                            ),
                            require_text: Some(grav.depoente_nome.clone()),
                            typed_text: String::new(),
                            action: ConfirmAction::DeleteRecording {
                                id: grav.id.clone(),
                                path: grav.arquivo_path.clone(),
                            },
                        });
                    }
                });
            });
            ui.add_space(4.0);
        }
    });
}

fn format_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}h {m:02}min")
    } else {
        format!("{m}min {s:02}s")
    }
}

fn format_bytes(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else {
        format!("{:.0} MB", bytes as f64 / MB as f64)
    }
}

fn open_in_system(path: &str) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(path).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = path;
    }
}
