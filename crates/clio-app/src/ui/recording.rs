//! Aba de gravação — painel principal com vídeo, identificação e controles.

use crate::app_state::{ActiveRecording, AppState, ConfirmAction, ConfirmDialog};
use crate::ui::main_layout::format_secs;
use clio_core::recording::RecordingState;
use clio_media::{DeviceEvent, PipelineCommand, PipelineEvent};
use egui::Ui;
use tracing::{info, warn};

/// Processa eventos do pipeline (chamado a cada frame, antes do rendering).
pub fn process_pipeline_events(state: &mut AppState) {
    let events: Vec<PipelineEvent> = if let Some(rec) = &state.active_recording {
        std::iter::from_fn(|| rec.pipeline.try_recv_event()).collect()
    } else {
        return;
    };

    for event in events {
        match event {
            PipelineEvent::DurationUpdate {
                secs_recorded,
                secs_paused,
            } => {
                if let Some(rec) = &mut state.active_recording {
                    rec.secs_recorded = secs_recorded;
                    rec.secs_paused = secs_paused;
                }
            }
            PipelineEvent::VuUpdate { mic, level } => {
                if let Some(rec) = &mut state.active_recording {
                    if mic == 1 {
                        rec.vu_mic1 = level;
                    } else {
                        rec.vu_mic2 = level;
                    }
                }
            }
            PipelineEvent::PreviewFrame {
                width,
                height,
                data,
            } => {
                if let Some(rec) = &mut state.active_recording {
                    rec.preview_frame = Some((width, height, data));
                }
            }
            PipelineEvent::DiskSpaceUpdate {
                minutes_remaining,
                bytes_available,
            } => {
                if let Some(rec) = &mut state.active_recording {
                    rec.disk_minutes_remaining = minutes_remaining;
                    rec.disk_bytes_available = bytes_available;
                }
                let thresh = state.config.gravacao.alerta_espaco_min_minutos as f64;
                if let Some(rec) = &state.active_recording {
                    if rec.disk_minutes_remaining < thresh {
                        let msg = format!(
                            "⚠ Espaço em disco: {:.0} min restantes",
                            rec.disk_minutes_remaining
                        );
                        warn!("{msg}");
                        // Adicionar aviso apenas uma vez por threshold
                        if let Some(r) = &mut state.active_recording {
                            if !r.warnings.iter().any(|w| w.contains("Espaço em disco")) {
                                r.warnings.push(msg);
                            }
                        }
                    }
                }
            }
            PipelineEvent::CameraLost { cam } => {
                let msg = format!("⚠ Câmera {} desconectada durante a gravação", cam);
                warn!("{msg}");
                state.push_toast(crate::app_state::Toast::error(&msg));
                if let Some(rec) = &mut state.active_recording {
                    rec.warnings.push(msg);
                }
            }
            PipelineEvent::MicLost { mic } => {
                let msg = format!("⚠ Microfone {} desconectado durante a gravação", mic);
                warn!("{msg}");
                state.push_toast(crate::app_state::Toast::error(&msg));
            }
            PipelineEvent::MicSilenceAlert { mic } => {
                let rotulo = if mic == 1 {
                    state.config.audio.mic1_rotulo.clone()
                } else {
                    state.config.audio.mic2_rotulo.clone()
                };
                let msg = format!("⚠ Microfone '{rotulo}' sem sinal há mais de 10 s");
                state.push_toast(crate::app_state::Toast::warning(&msg));
            }
            PipelineEvent::Stopped { .. } => {
                info!("Pipeline encerrado");
                // Limpeza feita pela lógica de finalização
            }
            PipelineEvent::Error(e) => {
                state.push_toast(crate::app_state::Toast::error(format!(
                    "Erro de gravação: {e}"
                )));
                if let Some(rec) = &mut state.active_recording {
                    rec.warnings.push(e);
                }
            }
            PipelineEvent::Started { encoder_used, .. } => {
                info!("Pipeline iniciado com encoder: {encoder_used}");
                state.push_toast(crate::app_state::Toast::success(format!(
                    "Gravação iniciada — encoder: {encoder_used}"
                )));
            }
            PipelineEvent::HashBlock(_) => {
                // Registrado no diário via clio-store (integração na fase c)
            }
        }
    }
}

/// Processa eventos de dispositivos.
pub fn process_device_events(state: &mut AppState) {
    while let Ok(ev) = state.device_monitor.event_rx.try_recv() {
        match ev {
            DeviceEvent::Added(dev) => {
                info!(device = %dev.display_name, "Dispositivo conectado");
            }
            DeviceEvent::Removed(id) => {
                warn!(device = %id, "Dispositivo removido");
            }
        }
    }
}

/// Renderiza a aba de gravação completa.
pub fn show_recording_tab(ui: &mut Ui, state: &mut AppState) {
    let is_recording = state
        .active_recording
        .as_ref()
        .map(|r| r.state.is_active())
        .unwrap_or(false);

    // ── Avisos de sessões pendentes ────────────────────────────────────────
    // (exibidos apenas enquanto o dialogo não for fechado — controlado em app.rs)

    egui::SidePanel::right("right_panel")
        .min_width(320.0)
        .max_width(400.0)
        .show_inside(ui, |ui| {
            right_panel(ui, state, is_recording);
        });

    egui::TopBottomPanel::bottom("control_strip")
        .min_height(80.0)
        .show_inside(ui, |ui| {
            control_strip(ui, state, is_recording);
        });

    // Área central: prévia do vídeo
    egui::CentralPanel::default().show_inside(ui, |ui| {
        video_preview(ui, state);
    });
}

fn video_preview(ui: &mut Ui, state: &mut AppState) {
    let avail = ui.available_size();

    // Checklist de pré-gravação (quando não está gravando)
    if state.active_recording.is_none() {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.heading("Prévia da câmera");
            ui.add_space(8.0);

            // Placeholder da prévia
            let preview_rect = egui::Rect::from_center_size(
                ui.cursor().center() + egui::vec2(0.0, 80.0),
                egui::vec2(
                    avail.x.min(640.0),
                    (avail.x.min(640.0) * 9.0 / 16.0).min(360.0),
                ),
            );
            ui.painter()
                .rect_filled(preview_rect, 4.0, egui::Color32::from_rgb(20, 20, 30));
            ui.painter().text(
                preview_rect.center(),
                egui::Align2::CENTER_CENTER,
                "Aguardando câmera…",
                egui::FontId::proportional(16.0),
                egui::Color32::from_gray(100),
            );

            ui.add_space(preview_rect.height() + 20.0);
            show_prechecklist(ui, state);
        });
    } else if let Some(rec) = &state.active_recording {
        // Prévia durante gravação (placeholder — frame real via egui_extras na fase b)
        let preview_w = avail.x;
        let preview_h = (preview_w * 9.0 / 16.0).min(avail.y - 20.0);
        let preview_rect =
            egui::Rect::from_min_size(ui.cursor().min, egui::vec2(preview_w, preview_h));
        ui.painter()
            .rect_filled(preview_rect, 0.0, egui::Color32::from_rgb(10, 10, 15));
        ui.painter().text(
            preview_rect.center(),
            egui::Align2::CENTER_CENTER,
            if rec.state == RecordingState::Paused {
                "⏸ PAUSADO"
            } else {
                "🔴 GRAVANDO"
            },
            egui::FontId::proportional(28.0),
            if rec.state == RecordingState::Paused {
                egui::Color32::YELLOW
            } else {
                egui::Color32::from_rgb(220, 40, 40)
            },
        );
        ui.allocate_rect(preview_rect, egui::Sense::hover());

        // Avisos sobrepostos
        if !rec.warnings.is_empty() {
            ui.colored_label(
                egui::Color32::YELLOW,
                format!("⚠ {}", rec.warnings.last().unwrap()),
            );
        }
    }
}

fn show_prechecklist(ui: &mut Ui, state: &AppState) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("✅ Verificações antes de iniciar").strong());
        ui.separator();

        let cameras = state.device_monitor.cameras();
        let mics = state.device_monitor.microphones();
        let draft = &state.session_draft;

        checklist_item(
            ui,
            !cameras.is_empty(),
            "Câmera 1 com imagem",
            "Nenhuma câmera detectada",
            true,
        );
        checklist_item(
            ui,
            !mics.is_empty(),
            "Microfone 1 com sinal",
            "Nenhum microfone detectado",
            true,
        );
        checklist_item(
            ui,
            !draft.depoente.nome.trim().is_empty(),
            "Nome do depoente preenchido",
            "Preencha o nome do depoente",
            true,
        );
        checklist_item(
            ui,
            !draft.processo.numero.trim().is_empty(),
            "Número dos autos preenchido",
            "Preencha o número dos autos",
            true,
        );

        // Verificação de espaço
        let disk_ok = clio_media::space_monitor::DiskSpace::read(
            &state
                .config
                .gravacao
                .pasta_destino
                .clone()
                .unwrap_or_else(|| std::path::PathBuf::from(".")),
        )
        .map(|d| d.suficiente_para_iniciar())
        .unwrap_or(true);
        checklist_item(
            ui,
            disk_ok,
            "Espaço em disco suficiente",
            "Espaço insuficiente",
            true,
        );
    });
}

fn checklist_item(ui: &mut Ui, ok: bool, label_ok: &str, label_fail: &str, blocking: bool) {
    let (icon, cor, texto) = if ok {
        ("✅", egui::Color32::GREEN, label_ok)
    } else if blocking {
        ("❌", egui::Color32::RED, label_fail)
    } else {
        ("⚠", egui::Color32::YELLOW, label_fail)
    };
    ui.horizontal(|ui| {
        ui.colored_label(cor, icon);
        ui.label(texto);
    });
}

fn right_panel(ui: &mut Ui, state: &mut AppState, is_recording: bool) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── Identificação ──────────────────────────────────────────────────
        ui.collapsing("📋 Identificação", |ui| {
            identification_form(ui, state, is_recording);
        });
        ui.add_space(4.0);

        // ── Participantes ──────────────────────────────────────────────────
        ui.collapsing("👥 Participantes e Defesa", |ui| {
            participants_form(ui, state, is_recording);
        });
        ui.add_space(4.0);

        // ── Dispositivos e Áudio ───────────────────────────────────────────
        ui.collapsing("🎙 Dispositivos e Áudio", |ui| {
            devices_form(ui, state);
        });
        ui.add_space(4.0);

        // ── Tarja ──────────────────────────────────────────────────────────
        ui.collapsing("🏷 Tarja probatória", |ui| {
            tarja_form(ui, state);
        });
    });
}

fn identification_form(ui: &mut Ui, state: &mut AppState, locked: bool) {
    ui.add_enabled_ui(!locked, |ui| {
        ui.label("Depoente:");
        let resp = ui.text_edit_singleline(&mut state.session_draft.depoente.nome);
        if resp.changed() && state.session_draft.depoente.nome.trim().is_empty() {
            state
                .session_draft
                .validation_msgs
                .insert("depoente.nome".into(), "Campo obrigatório".into());
        } else {
            state.session_draft.validation_msgs.remove("depoente.nome");
        }
        if let Some(msg) = state.session_draft.validation_msgs.get("depoente.nome") {
            ui.colored_label(egui::Color32::RED, msg);
        }

        ui.label("Qualificação:");
        egui::ComboBox::from_id_salt("qualificacao_combo")
            .selected_text(state.session_draft.depoente.qualificacao.label())
            .show_ui(ui, |ui| {
                use clio_core::models::Qualificacao;
                for qual in [
                    Qualificacao::Testemunha,
                    Qualificacao::Vitima,
                    Qualificacao::Investigado,
                    Qualificacao::Reu,
                    Qualificacao::Perito,
                ] {
                    let selected = state.session_draft.depoente.qualificacao == qual;
                    if ui.selectable_label(selected, qual.label()).clicked() {
                        state.session_draft.depoente.qualificacao = qual;
                    }
                }
            });

        ui.label("Documento (CPF/RG, opcional):");
        let mut doc = state
            .session_draft
            .depoente
            .documento
            .clone()
            .unwrap_or_default();
        if ui.text_edit_singleline(&mut doc).changed() {
            state.session_draft.depoente.documento = if doc.is_empty() { None } else { Some(doc) };
        }

        ui.separator();
        ui.label(format!("{}: ", state.config.instituicao.orgao));
        ui.label(&state.config.instituicao.autoridade_titulo);
        ui.label(&state.config.instituicao.autoridade_nome);

        ui.separator();
        ui.label("Nº dos Autos:");
        let resp = ui.text_edit_singleline(&mut state.session_draft.processo.numero);
        if resp.changed() && state.session_draft.processo.numero.trim().is_empty() {
            state
                .session_draft
                .validation_msgs
                .insert("processo.numero".into(), "Campo obrigatório".into());
        } else {
            state
                .session_draft
                .validation_msgs
                .remove("processo.numero");
        }
        if let Some(msg) = state.session_draft.validation_msgs.get("processo.numero") {
            ui.colored_label(egui::Color32::RED, msg);
        }

        ui.label("Tipo:");
        egui::ComboBox::from_id_salt("tipo_processo_combo")
            .selected_text(state.session_draft.processo.tipo.label())
            .show_ui(ui, |ui| {
                use clio_core::models::TipoProcesso;
                for tipo in [
                    TipoProcesso::Autos,
                    TipoProcesso::Inquerito,
                    TipoProcesso::ProcedimentoInvestigatorio,
                ] {
                    let selected = state.session_draft.processo.tipo == tipo;
                    if ui.selectable_label(selected, tipo.label()).clicked() {
                        state.session_draft.processo.tipo = tipo;
                    }
                }
            });
    });
}

fn participants_form(ui: &mut Ui, state: &mut AppState, locked: bool) {
    ui.add_enabled_ui(!locked, |ui| {
        ui.label(egui::RichText::new("Defesas").strong());

        let mut to_remove: Option<usize> = None;
        for (i, defesa) in state.session_draft.defesas.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Nome:");
                    ui.text_edit_singleline(&mut defesa.nome);
                    if ui.small_button("✕").clicked() {
                        to_remove = Some(i);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("OAB:");
                    let mut oab = defesa.oab.clone().unwrap_or_default();
                    let resp = ui.text_edit_singleline(&mut oab);
                    if resp.changed() {
                        // Validar OAB sem bloquear
                        if !oab.is_empty() {
                            if let Err(_e) = clio_core::validation::validar_oab(&oab) {
                                ui.colored_label(egui::Color32::YELLOW, "⚠ Formato OAB inválido");
                            }
                        }
                        defesa.oab = if oab.is_empty() { None } else { Some(oab) };
                    }
                    ui.label("Representa:");
                    let mut rep = defesa.representa.clone().unwrap_or_default();
                    if ui.text_edit_singleline(&mut rep).changed() {
                        defesa.representa = if rep.is_empty() { None } else { Some(rep) };
                    }
                });
            });
        }
        if let Some(i) = to_remove {
            state.session_draft.defesas.remove(i);
        }

        if ui.button("+ Adicionar Defesa").clicked() {
            state
                .session_draft
                .defesas
                .push(clio_core::models::Defesa::new(String::new()));
        }
    });
}

fn devices_form(ui: &mut Ui, state: &mut AppState) {
    let cameras = state.device_monitor.cameras();
    let mics = state.device_monitor.microphones();

    ui.label(egui::RichText::new("Câmeras").strong());
    ui.label(format!("{} câmera(s) disponível(eis)", cameras.len()));
    for cam in &cameras {
        ui.label(format!("  • {}", cam.display_name));
    }

    ui.separator();
    ui.label(egui::RichText::new("Microfones").strong());
    for (i, label) in [
        (1u8, state.config.audio.mic1_rotulo.clone()),
        (2u8, state.config.audio.mic2_rotulo.clone()),
    ] {
        ui.horizontal(|ui| {
            ui.label(format!("Mic {i} — {label}:"));
            if mics.is_empty() {
                ui.colored_label(egui::Color32::RED, "nenhum detectado");
            } else {
                ui.label(mics.first().map(|m| m.display_name.as_str()).unwrap_or("?"));
            }
        });

        // Medidor VU
        if let Some(rec) = &state.active_recording {
            let level = if i == 1 { rec.vu_mic1 } else { rec.vu_mic2 };
            let bar_w = 160.0;
            let bar_h = 10.0;
            let (rect, _) = ui.allocate_exact_size(egui::vec2(bar_w, bar_h), egui::Sense::hover());
            let fill = if level.clipping {
                egui::Color32::RED
            } else if level.rms_normalized() > 0.8 {
                egui::Color32::YELLOW
            } else {
                egui::Color32::GREEN
            };
            ui.painter().rect_filled(
                egui::Rect::from_min_size(
                    rect.min,
                    egui::vec2(bar_w * level.rms_normalized(), bar_h),
                ),
                0.0,
                fill,
            );
            ui.painter().rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(1.0_f32, egui::Color32::GRAY),
                egui::StrokeKind::Inside,
            );
        }
    }
}

fn tarja_form(ui: &mut Ui, state: &mut AppState) {
    let tarja = &mut state.config.video.tarja;

    ui.horizontal(|ui| {
        ui.label("Transparência do fundo:");
        let old_val = tarja.transparencia_fundo_pct;
        ui.add(
            egui::Slider::new(&mut tarja.transparencia_fundo_pct, 0u8..=100)
                .suffix("%")
                .text(""),
        );
        if old_val != tarja.transparencia_fundo_pct {
            state.config_dirty = true;
            // Verificar limiar de aviso
            state.tarja_aviso_transparencia = tarja.transparencia_fundo_pct > 60;
        }
    });

    if state.tarja_aviso_transparencia {
        ui.colored_label(
            egui::Color32::YELLOW,
            "⚠ Fundo muito transparente pode reduzir a legibilidade do carimbo como prova.",
        );
    }

    // Predefinições
    ui.horizontal(|ui| {
        for (label, val) in [
            ("Opaco", 0u8),
            ("Padrão", 6),
            ("Translúcido", 50),
            ("Sem fundo", 100),
        ] {
            if ui.small_button(label).clicked() {
                tarja.transparencia_fundo_pct = val;
                state.config_dirty = true;
                state.tarja_aviso_transparencia = val > 60;
            }
        }
    });

    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Posição:");
        if ui
            .selectable_label(
                tarja.posicao == clio_config::schema::PosicaoTarja::Inferior,
                "Inferior",
            )
            .clicked()
        {
            tarja.posicao = clio_config::schema::PosicaoTarja::Inferior;
            state.config_dirty = true;
        }
        if ui
            .selectable_label(
                tarja.posicao == clio_config::schema::PosicaoTarja::Superior,
                "Superior",
            )
            .clicked()
        {
            tarja.posicao = clio_config::schema::PosicaoTarja::Superior;
            state.config_dirty = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Altura:");
        for altura in [
            clio_config::schema::AlturaTarja::Compacta,
            clio_config::schema::AlturaTarja::Normal,
            clio_config::schema::AlturaTarja::Ampla,
        ] {
            if ui
                .selectable_label(tarja.altura == altura, altura.label())
                .clicked()
            {
                tarja.altura = altura;
                state.config_dirty = true;
            }
        }
    });

    ui.checkbox(&mut tarja.mostrar_defesa, "Exibir linha de defesa na tarja");
}

fn control_strip(ui: &mut Ui, state: &mut AppState, is_recording: bool) {
    ui.horizontal_centered(|ui| {
        ui.add_space(16.0);

        if !is_recording {
            // Botão Iniciar
            let can_start = state.session_draft.is_valid_to_start()
                && !state.device_monitor.cameras().is_empty();

            let blocking = state.session_draft.blocking_issues();

            let btn = egui::Button::new(
                egui::RichText::new("▶ Iniciar Gravação  (Ctrl+Enter)")
                    .size(16.0)
                    .strong(),
            )
            .min_size(egui::vec2(240.0, 52.0));

            let resp = ui.add_enabled(can_start, btn);
            if resp.clicked() {
                start_recording(state);
            }
            if !blocking.is_empty() {
                resp.on_hover_text(blocking.join("\n"));
            }
        } else {
            let rec_state = state
                .active_recording
                .as_ref()
                .map(|r| r.state)
                .unwrap_or(RecordingState::Idle);

            // Botão Pausar / Retomar
            let (pause_label, pause_action) = if rec_state == RecordingState::Paused {
                ("▶ Retomar  (Espaço)", true)
            } else {
                ("⏸ Pausar (Recesso)  (Espaço)", false)
            };

            if ui
                .add(
                    egui::Button::new(egui::RichText::new(pause_label).size(14.0))
                        .min_size(egui::vec2(200.0, 52.0)),
                )
                .clicked()
            {
                toggle_pause(state, pause_action);
            }

            ui.add_space(16.0);

            // Botão Finalizar — separado visualmente
            let finalizar_btn = egui::Button::new(
                egui::RichText::new("⏹ Finalizar  (Ctrl+Shift+F)")
                    .size(14.0)
                    .color(egui::Color32::from_rgb(255, 100, 100)),
            )
            .min_size(egui::vec2(200.0, 52.0))
            .fill(egui::Color32::from_rgb(80, 20, 20));

            if ui.add(finalizar_btn).clicked() {
                state.confirm_dialog = Some(ConfirmDialog {
                    title: "Finalizar gravação".to_string(),
                    message: format!(
                        "Deseja finalizar a gravação?\nDuração: {}",
                        format_secs(
                            state
                                .active_recording
                                .as_ref()
                                .map(|r| r.secs_recorded)
                                .unwrap_or(0)
                        )
                    ),
                    require_text: None,
                    typed_text: String::new(),
                    action: ConfirmAction::StopRecording,
                });
            }

            // Cronômetro grande
            ui.add_space(24.0);
            let cronometro = format_secs(
                state
                    .active_recording
                    .as_ref()
                    .map(|r| r.secs_recorded)
                    .unwrap_or(0),
            );
            ui.label(
                egui::RichText::new(&cronometro)
                    .size(32.0)
                    .strong()
                    .monospace()
                    .color(if rec_state == RecordingState::Paused {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::WHITE
                    }),
            );
        }

        // Atalhos de teclado
        ui.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Enter) && !is_recording {
                if state.session_draft.is_valid_to_start() {
                    start_recording(state);
                }
            }
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::F) && is_recording
            {
                if state.confirm_dialog.is_none() {
                    state.confirm_dialog = Some(ConfirmDialog {
                        title: "Finalizar gravação".to_string(),
                        message: "Finalizar a gravação?".to_string(),
                        require_text: None,
                        typed_text: String::new(),
                        action: ConfirmAction::StopRecording,
                    });
                }
            }
            if i.key_pressed(egui::Key::Space) && is_recording {
                let paused = state
                    .active_recording
                    .as_ref()
                    .map(|r| r.state == RecordingState::Paused)
                    .unwrap_or(false);
                toggle_pause(state, paused);
            }
        });
    });
}

fn start_recording(state: &mut AppState) {
    info!("Iniciando gravação...");
    let handle = clio_media::pipeline::spawn_pipeline();

    let output_dir = state
        .config
        .gravacao
        .pasta_destino
        .clone()
        .unwrap_or_else(|| {
            directories::UserDirs::new()
                .and_then(|d| d.video_dir().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("Clio")
        });

    let inicio =
        time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());

    let filename = clio_core::filename::gerar_nome_arquivo(
        &state.session_draft.depoente.nome,
        &state.session_draft.processo.numero,
        &inicio,
        "mkv",
    )
    .unwrap_or_else(|_| "oitiva.mkv".to_string());

    let session_dir = output_dir.join(
        clio_core::filename::gerar_nome_pasta(
            &state.session_draft.depoente.nome,
            &state.session_draft.processo.numero,
            &inicio,
        )
        .unwrap_or_else(|_| "oitiva".to_string()),
    );
    let output_path = session_dir.join(&filename);

    let _ = std::fs::create_dir_all(&session_dir);

    let cam1 = state.device_monitor.cameras().into_iter().next();
    let mic1 = state.device_monitor.microphones().into_iter().next();
    let mic2 = if state.config.audio.mic2_ativo_por_padrao {
        state.device_monitor.microphones().into_iter().nth(1)
    } else {
        None
    };

    let _ = handle.send(PipelineCommand::Start {
        output_path: output_path.clone(),
        cam1,
        cam2: None,
        mic1,
        mic2,
        layout: state.config.video.layout_padrao.clone(),
        modo_audio: state.config.audio.modo.clone(),
        mic1_ganho_db: state.config.audio.mic1_ganho_db,
        mic2_ganho_db: state.config.audio.mic2_ganho_db,
        bitrate_video_kbps: state.config.gravacao.video_bitrate_kbps,
        bitrate_audio_kbps: state.config.gravacao.audio_bitrate_kbps,
    });

    // Ativar wake lock
    let _wake = clio_media::wake_lock::WakeLock::acquire();

    state.active_recording = Some(ActiveRecording {
        session_id: uuid::Uuid::new_v4().to_string(),
        state: RecordingState::Recording,
        started_at: inicio,
        output_path,
        secs_recorded: 0,
        secs_paused: 0,
        pipeline: handle,
        preview_frame: None,
        vu_mic1: Default::default(),
        vu_mic2: Default::default(),
        mic2_active: false,
        mic1_muted: false,
        mic2_muted: false,
        layout: state.config.video.layout_padrao.clone(),
        disk_minutes_remaining: 999.0,
        disk_bytes_available: 0,
        warnings: Vec::new(),
    });
}

fn toggle_pause(state: &mut AppState, resume: bool) {
    if let Some(rec) = &mut state.active_recording {
        if resume {
            let _ = rec.pipeline.send(PipelineCommand::Resume);
            rec.state = RecordingState::Recording;
            info!("Gravação retomada");
        } else {
            let _ = rec.pipeline.send(PipelineCommand::Pause);
            rec.state = RecordingState::Paused;
            info!("Gravação pausada");
        }
    }
}
