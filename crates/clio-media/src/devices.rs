//! Enumeração e monitoramento de câmeras e microfones via GStreamer DeviceMonitor.
//!
//! Reage a eventos de conexão/desconexão em tempo real sem interromper gravações em andamento.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use crossbeam_channel::{Sender, Receiver, unbounded};
use tracing::{info, warn};
use crate::error::MediaResult;

/// Tipo de dispositivo de mídia.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceKind {
    Camera,
    Microphone,
}

/// Informações de um dispositivo de captura.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Identificador estável do SO (usado para lembrar o último dispositivo).
    pub id: String,
    /// Nome amigável exibido na UI.
    pub display_name: String,
    pub kind: DeviceKind,
    /// Nome do elemento GStreamer fonte (ex.: "ksvideosrc", "wasapi2src").
    pub gst_element: String,
    /// Propriedades adicionais para o elemento (ex.: `device-index`, `device`).
    pub gst_props: std::collections::HashMap<String, String>,
}

/// Evento de mudança de dispositivo.
#[derive(Debug, Clone)]
pub enum DeviceEvent {
    Added(DeviceInfo),
    Removed(String), // id do dispositivo
}

/// Monitor de dispositivos — lista e monitora em tempo real.
pub struct DeviceMonitor {
    cameras: Arc<RwLock<Vec<DeviceInfo>>>,
    microphones: Arc<RwLock<Vec<DeviceInfo>>>,
    event_tx: Sender<DeviceEvent>,
    pub event_rx: Receiver<DeviceEvent>,
}

impl DeviceMonitor {
    pub fn new() -> Self {
        let (tx, rx) = unbounded();
        Self {
            cameras: Arc::new(RwLock::new(Vec::new())),
            microphones: Arc::new(RwLock::new(Vec::new())),
            event_tx: tx,
            event_rx: rx,
        }
    }

    /// Inicializa o monitor com enumeração inicial dos dispositivos.
    /// Em builds sem a feature `gst`, popula uma lista vazia.
    pub fn start(&self) -> MediaResult<()> {
        #[cfg(feature = "gst")]
        self.start_gst()?;

        #[cfg(not(feature = "gst"))]
        {
            warn!("GStreamer não compilado — monitor de dispositivos desativado");
            self.populate_stub_devices();
        }
        Ok(())
    }

    /// Lista câmeras disponíveis no momento.
    pub fn cameras(&self) -> Vec<DeviceInfo> {
        self.cameras.read().clone()
    }

    /// Lista microfones disponíveis no momento.
    pub fn microphones(&self) -> Vec<DeviceInfo> {
        self.microphones.read().clone()
    }

    /// Localiza um dispositivo pelo ID estável.
    pub fn find_by_id(&self, id: &str) -> Option<DeviceInfo> {
        let all: Vec<DeviceInfo> = self
            .cameras()
            .into_iter()
            .chain(self.microphones())
            .collect();
        all.into_iter().find(|d| d.id == id)
    }

    #[cfg(not(feature = "gst"))]
    fn populate_stub_devices(&self) {
        // Dispositivos fictícios para desenvolvimento sem GStreamer instalado.
        let cam = DeviceInfo {
            id: "stub-cam-0".to_string(),
            display_name: "Câmera de Teste (Stub)".to_string(),
            kind: DeviceKind::Camera,
            gst_element: "videotestsrc".to_string(),
            gst_props: Default::default(),
        };
        let mic = DeviceInfo {
            id: "stub-mic-0".to_string(),
            display_name: "Microfone de Teste (Stub)".to_string(),
            kind: DeviceKind::Microphone,
            gst_element: "audiotestsrc".to_string(),
            gst_props: Default::default(),
        };
        self.cameras.write().push(cam.clone());
        self.microphones.write().push(mic.clone());
        let _ = self.event_tx.send(DeviceEvent::Added(cam));
        let _ = self.event_tx.send(DeviceEvent::Added(mic));
    }

    #[cfg(feature = "gst")]
    fn start_gst(&self) -> MediaResult<()> {
        use gstreamer as gst;
        gst::init().map_err(|e| crate::error::MediaError::GstInit(e.to_string()))?;

        let monitor = gst::DeviceMonitor::new();
        // Filtrar apenas vídeo e áudio de captura
        monitor.add_filter(Some("Video/Source"), None);
        monitor.add_filter(Some("Audio/Source"), None);

        // Enumeração inicial
        let initial_devices = monitor.devices();
        for dev in initial_devices {
            if let Some(info) = gst_device_to_info(&dev) {
                match info.kind {
                    DeviceKind::Camera => self.cameras.write().push(info.clone()),
                    DeviceKind::Microphone => self.microphones.write().push(info.clone()),
                }
                let _ = self.event_tx.send(DeviceEvent::Added(info));
            }
        }

        // Monitoramento em thread própria
        let cameras = Arc::clone(&self.cameras);
        let microphones = Arc::clone(&self.microphones);
        let tx = self.event_tx.clone();

        monitor.start().map_err(|e| crate::error::MediaError::GstInit(e.to_string()))?;
        let bus = monitor.bus();

        std::thread::Builder::new()
            .name("clio-device-monitor".into())
            .spawn(move || {
                loop {
                    if let Some(msg) = bus.timed_pop(gst::ClockTime::from_seconds(1)) {
                        match msg.view() {
                            gst::MessageView::DeviceAdded(da) => {
                                if let Some(info) = gst_device_to_info(&da.device()) {
                                    info!(device = %info.display_name, "Dispositivo conectado");
                                    match info.kind {
                                        DeviceKind::Camera => cameras.write().push(info.clone()),
                                        DeviceKind::Microphone => microphones.write().push(info.clone()),
                                    }
                                    let _ = tx.send(DeviceEvent::Added(info));
                                }
                            }
                            gst::MessageView::DeviceRemoved(dr) => {
                                let dev = dr.device();
                                let name = dev.display_name().to_string();
                                warn!(device = %name, "Dispositivo desconectado");
                                cameras.write().retain(|d| d.display_name != name);
                                microphones.write().retain(|d| d.display_name != name);
                                let _ = tx.send(DeviceEvent::Removed(name));
                            }
                            _ => {}
                        }
                    }
                }
            })
            .map_err(|e| crate::error::MediaError::Internal(e.to_string()))?;

        info!("Monitor de dispositivos GStreamer iniciado");
        Ok(())
    }
}

impl Default for DeviceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "gst")]
fn gst_device_to_info(dev: &gstreamer::Device) -> Option<DeviceInfo> {
    let class = dev.device_class().to_string();
    let kind = if class.contains("Video") {
        DeviceKind::Camera
    } else if class.contains("Audio") {
        DeviceKind::Microphone
    } else {
        return None;
    };

    // Obter o nome do elemento fonte
    let caps = dev.caps()?;
    let element_name = match kind {
        DeviceKind::Camera => "ksvideosrc",    // Windows
        DeviceKind::Microphone => "wasapi2src", // Windows
    };

    Some(DeviceInfo {
        id: dev.display_name().to_string(),
        display_name: dev.display_name().to_string(),
        kind,
        gst_element: element_name.to_string(),
        gst_props: Default::default(),
    })
}
