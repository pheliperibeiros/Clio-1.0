//! clio-media — GStreamer: dispositivos, pipeline, compositor, tarja, VU, wake lock.
//!
//! # Arquitetura do pipeline
//!
//! ```text
//! cam1 (v4l2src / ksvideosrc)  ──┐
//!                                  ├─ compositor ──► videoconvert ──► encoder ──┐
//! cam2 (opcional)              ──┘                                               │
//!                                                                                ├─ matroskamux ──► filesink
//! appsrc (tarja RGBA)          ──────────────────────────────────────────────────┘ (overlay no compositor)
//!
//! mic1 (pulsesrc / wasapi2src) ──► audioconvert ──► audiorate ──► audiomixer ──► faixa mix
//!                                                                              └──► faixa mic1 (modo faixas_separadas)
//! mic2 (opcional)              ──► audioconvert ──► audiorate ──┘               └──► faixa mic2
//! ```
//!
//! Nota: O pipeline GStreamer real está por trás da feature flag `gst`.
//! Sem a feature, o crate compila com stubs que permitem desenvolver a UI e rodar testes unitários.
#![deny(unsafe_code)]
#![warn(clippy::all)]

pub mod devices;
pub mod error;
pub mod overlay;
pub mod pipeline;
pub mod space_monitor;
pub mod vu;
pub mod wake_lock;

pub use devices::{DeviceEvent, DeviceInfo, DeviceKind, DeviceMonitor};
pub use error::MediaError;
pub use overlay::{OverlayParams, OverlayRenderer};
pub use pipeline::{PipelineCommand, PipelineEvent, PipelineHandle};
pub use vu::VuLevel;
