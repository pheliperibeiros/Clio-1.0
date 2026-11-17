//! Pipeline GStreamer de gravação resiliente.
//!
//! Esta interface é independente da feature `gst`: os tipos `PipelineCommand` e
//! `PipelineEvent` são sempre definidos; a implementação real está por trás de `#[cfg(feature = "gst")]`.
//!
//! # Contêiner escolhido: Matroska (MKV) em modo streamable
//!
//! Resultado do spike (Etapa 0):
//! - `mp4mux` fragmentado: arquivo abre no VLC mas NÃO abre no Windows Media Player sem índice.
//!   Requer `qtfaststart` ou remux para mover o `moov` atom para o início após a queda.
//! - `matroskamux streamable=true`: arquivo abre corretamente em VLC e Windows Media Player
//!   mesmo após `TerminateProcess`. O MKV sem índice reproduz do início ao ponto do último
//!   cluster gravado antes da queda. Suporte nativo a múltiplas faixas de áudio.
//!
//! **Escolha: MKV streamable.** MP4 fragmentado pode ser gerado como cópia derivada
//! opcionalmente (para facilitar scrubbing no player), mas o arquivo-mestre é MKV.
//!
//! # Prioridade de encoders H.264
//! 1. `nvh264enc` (NVIDIA NVENC)
//! 2. `qsvh264enc` (Intel Quick Sync)
//! 3. `amfh264enc` (AMD AMF)
//! 4. `mfh264enc` (Windows Media Foundation)
//! 5. `openh264enc` (Cisco BSD — padrão no CI)
//! 6. `x264enc` (GPL — apenas se licença aceita e documentada)
//!
//! # Deriva de clock entre dois microfones USB
//! Cada ramo de microfone tem `audiorate` antes do `audiomixer`. O `audiorate`
//! insere/descarta amostras para corrigir timestamps inconsistentes, eliminando
//! o drift de clock entre dispositivos USB independentes.

use crossbeam_channel::{Sender, Receiver, unbounded};
use clio_config::schema::{Layout, ModoAudio};
use crate::devices::DeviceInfo;
use crate::error::{MediaError, MediaResult};
use crate::overlay::OverlayParams;

// ─── Comandos enviados pelo app ao pipeline ───────────────────────────────────

#[derive(Debug)]
pub enum PipelineCommand {
    /// Iniciar gravação.
    Start {
        output_path: std::path::PathBuf,
        cam1: Option<DeviceInfo>,
        cam2: Option<DeviceInfo>,
        mic1: Option<DeviceInfo>,
        mic2: Option<DeviceInfo>,
        layout: Layout,
        modo_audio: ModoAudio,
        mic1_ganho_db: f32,
        mic2_ganho_db: f32,
        bitrate_video_kbps: u32,
        bitrate_audio_kbps: u32,
    },
    /// Pausar (recesso) — pipeline continua mas encoder descarta frames de vídeo.
    Pause,
    /// Retomar após pausa.
    Resume,
    /// Encerrar a gravação com flush completo.
    Stop,
    /// Atualizar o frame da tarja (injetado via appsrc).
    UpdateOverlay(OverlayParams),
    /// Trocar o layout de composição durante a gravação.
    SetLayout(Layout),
    /// Habilitar/desabilitar microfone 2 durante a gravação.
    SetMic2Active(bool),
    /// Ajustar ganho de um microfone (1 ou 2) em dB.
    SetMicGain { mic: u8, gain_db: f32 },
    /// Mutar/desmutar um microfone.
    SetMicMute { mic: u8, muted: bool },
}

// ─── Eventos enviados pelo pipeline ao app ────────────────────────────────────

#[derive(Debug, Clone)]
pub enum PipelineEvent {
    /// Pipeline iniciado com sucesso.
    Started {
        encoder_used: String,
        actual_bitrate_kbps: u32,
    },
    /// Novo bloco de hash disponível (N MiB gravados).
    HashBlock(clio_custody::chain::ChainBlock),
    /// Nível VU atualizado para o microfone.
    VuUpdate { mic: u8, level: crate::vu::VuLevel },
    /// Frame de prévia disponível (RGBA, resolução reduzida).
    PreviewFrame { width: u32, height: u32, data: Vec<u8> },
    /// Alerta: câmera desconectada durante gravação.
    CameraLost { cam: u8 },
    /// Alerta: microfone desconectado durante gravação.
    MicLost { mic: u8 },
    /// Alerta: silêncio prolongado no microfone.
    MicSilenceAlert { mic: u8 },
    /// Espaço em disco: minutos restantes.
    DiskSpaceUpdate { minutes_remaining: f64, bytes_available: u64 },
    /// Pipeline encerrado normalmente.
    Stopped { file_size_bytes: u64 },
    /// Erro irrecuperável.
    Error(String),
    /// Duração gravada até o momento (atualizada a cada segundo).
    DurationUpdate { secs_recorded: u64, secs_paused: u64 },
}

/// Handle de controle do pipeline de gravação.
pub struct PipelineHandle {
    pub cmd_tx: Sender<PipelineCommand>,
    pub event_rx: Receiver<PipelineEvent>,
}

impl PipelineHandle {
    pub fn send(&self, cmd: PipelineCommand) -> MediaResult<()> {
        self.cmd_tx.send(cmd).map_err(|_| MediaError::ChannelClosed)
    }

    pub fn try_recv_event(&self) -> Option<PipelineEvent> {
        self.event_rx.try_recv().ok()
    }
}

/// Lança o pipeline de gravação em uma thread dedicada.
/// Retorna o handle de controle imediatamente; o pipeline aguarda o comando `Start`.
pub fn spawn_pipeline() -> PipelineHandle {
    let (cmd_tx, cmd_rx) = unbounded::<PipelineCommand>();
    let (event_tx, event_rx) = unbounded::<PipelineEvent>();

    std::thread::Builder::new()
        .name("clio-pipeline".into())
        .spawn(move || {
            pipeline_thread(cmd_rx, event_tx);
        })
        .expect("Falha ao criar thread do pipeline");

    PipelineHandle { cmd_tx, event_rx }
}

// ─── Implementação da thread do pipeline ─────────────────────────────────────

#[cfg(feature = "gst")]
fn pipeline_thread(
    cmd_rx: Receiver<PipelineCommand>,
    event_tx: Sender<PipelineEvent>,
) {
    use gstreamer as gst;
    use gstreamer::prelude::*;
    use tracing::{error, info, warn};

    if let Err(e) = gst::init() {
        let _ = event_tx.send(PipelineEvent::Error(format!("GStreamer init: {e}")));
        return;
    }

    // Aguardar comando Start
    let start_cmd = loop {
        match cmd_rx.recv() {
            Ok(PipelineCommand::Start { .. }) => break,
            Ok(_) => continue,
            Err(_) => return,
        }
    };

    // Montar pipeline baseado nos parâmetros
    // (implementação completa na fase (b))
    let _ = event_tx.send(PipelineEvent::Error(
        "Pipeline GStreamer completo implementado na fase (b)".to_string()
    ));
}

#[cfg(not(feature = "gst"))]
fn pipeline_thread(
    cmd_rx: Receiver<PipelineCommand>,
    event_tx: Sender<PipelineEvent>,
) {
    use std::time::{Duration, Instant};
    use tracing::{info, warn};

    warn!("GStreamer não compilado — pipeline em modo stub (apenas para testes de UI)");

    let mut recording = false;
    let mut paused = false;
    let mut start_time: Option<Instant> = None;
    let mut paused_secs: u64 = 0;
    let mut pause_start: Option<Instant> = None;
    let mut output_path: Option<std::path::PathBuf> = None;

    loop {
        // Processar comandos disponíveis
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                PipelineCommand::Start { output_path: path, .. } => {
                    recording = true;
                    start_time = Some(Instant::now());
                    output_path = Some(path.clone());
                    info!("Pipeline stub: iniciando gravação em {:?}", path);
                    // Criar arquivo vazio para simular
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::File::create(&path);
                    let _ = event_tx.send(PipelineEvent::Started {
                        encoder_used: "stub-encoder".to_string(),
                        actual_bitrate_kbps: 4000,
                    });
                }
                PipelineCommand::Pause => {
                    paused = true;
                    pause_start = Some(Instant::now());
                }
                PipelineCommand::Resume => {
                    if let Some(ps) = pause_start.take() {
                        paused_secs += ps.elapsed().as_secs();
                    }
                    paused = false;
                }
                PipelineCommand::Stop => {
                    recording = false;
                    let _ = event_tx.send(PipelineEvent::Stopped { file_size_bytes: 0 });
                    return;
                }
                PipelineCommand::UpdateOverlay(_) => {}
                PipelineCommand::SetLayout(_) => {}
                PipelineCommand::SetMic2Active(_) => {}
                PipelineCommand::SetMicGain { .. } => {}
                PipelineCommand::SetMicMute { .. } => {}
            }
        }

        // Emitir eventos periódicos quando em gravação
        if recording {
            if let Some(t) = start_time {
                let elapsed = t.elapsed().as_secs();
                let cur_paused = paused_secs
                    + pause_start.map(|p| p.elapsed().as_secs()).unwrap_or(0);
                let _ = event_tx.send(PipelineEvent::DurationUpdate {
                    secs_recorded: elapsed.saturating_sub(cur_paused),
                    secs_paused: cur_paused,
                });
                // Simular espaço em disco
                let _ = event_tx.send(PipelineEvent::DiskSpaceUpdate {
                    minutes_remaining: 300.0,
                    bytes_available: 10 * 1024 * 1024 * 1024,
                });
            }
        }

        std::thread::sleep(Duration::from_millis(500));
    }
}

/// Detecta o melhor encoder H.264 disponível no sistema.
/// Retorna o nome do elemento GStreamer e uma descrição.
#[cfg(feature = "gst")]
pub fn detect_best_h264_encoder() -> Option<(&'static str, &'static str)> {
    use gstreamer::prelude::*;
    let candidates = [
        ("nvh264enc",   "NVIDIA NVENC"),
        ("qsvh264enc",  "Intel Quick Sync"),
        ("amfh264enc",  "AMD AMF"),
        ("mfh264enc",   "Windows Media Foundation"),
        ("openh264enc", "OpenH264 (Cisco BSD)"),
        // x264enc omitido intencionalmente (GPL); adicionar apenas com decisão documentada.
    ];
    for (elem, desc) in candidates {
        if gstreamer::ElementFactory::find(elem).is_some() {
            return Some((elem, desc));
        }
    }
    None
}

#[cfg(not(feature = "gst"))]
pub fn detect_best_h264_encoder() -> Option<(&'static str, &'static str)> {
    Some(("stub-encoder", "Stub (sem GStreamer)"))
}
