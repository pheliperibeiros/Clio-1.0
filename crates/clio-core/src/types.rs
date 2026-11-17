//! Tipos básicos compartilhados em todo o domínio.

use serde::{Deserialize, Serialize};
use std::fmt;
use time::OffsetDateTime;

/// Identificador único de uma sessão de gravação.
pub type SessionId = uuid::Uuid;

/// Identificador único de um participante.
pub type ParticipantId = uuid::Uuid;

/// Instant representado como `OffsetDateTime` (fuso local no momento da criação).
pub type Timestamp = OffsetDateTime;

/// Codec de vídeo negociado em runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoCodec {
    H264,
    H265,
}

impl fmt::Display for VideoCodec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::H264 => write!(f, "H.264"),
            Self::H265 => write!(f, "H.265"),
        }
    }
}

/// Codec de áudio negociado em runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioCodec {
    Aac,
    Opus,
}

impl fmt::Display for AudioCodec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Aac => write!(f, "AAC"),
            Self::Opus => write!(f, "Opus"),
        }
    }
}

/// Contêiner de vídeo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Container {
    Mkv,
    Mp4Fragmented,
}

impl fmt::Display for Container {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mkv => write!(f, "Matroska (MKV)"),
            Self::Mp4Fragmented => write!(f, "MP4 fragmentado"),
        }
    }
}

/// Resolução de vídeo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Resolution {
    pub const FULL_HD: Self = Self {
        width: 1920,
        height: 1080,
    };
    pub const HD: Self = Self {
        width: 1280,
        height: 720,
    };
    pub const PREVIEW: Self = Self {
        width: 960,
        height: 540,
    };
}

impl fmt::Display for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}×{}", self.width, self.height)
    }
}

/// Parâmetros técnicos reais do pipeline (preenchidos após negociação).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineParams {
    pub resolution: Resolution,
    pub fps: u32,
    pub video_codec: VideoCodec,
    pub audio_codec: AudioCodec,
    pub container: Container,
    pub video_bitrate_kbps: u32,
    pub audio_bitrate_kbps: u32,
}

impl Default for PipelineParams {
    fn default() -> Self {
        Self {
            resolution: Resolution::FULL_HD,
            fps: 30,
            video_codec: VideoCodec::H264,
            audio_codec: AudioCodec::Aac,
            container: Container::Mkv,
            video_bitrate_kbps: 4000,
            audio_bitrate_kbps: 128,
        }
    }
}

/// Duração em segundos inteiros (evita float nos cálculos de custódia).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct DurationSecs(pub u64);

impl DurationSecs {
    pub fn from_std(d: std::time::Duration) -> Self {
        Self(d.as_secs())
    }
    pub fn as_secs(self) -> u64 {
        self.0
    }
}

impl fmt::Display for DurationSecs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let h = self.0 / 3600;
        let m = (self.0 % 3600) / 60;
        let s = self.0 % 60;
        if h > 0 {
            write!(f, "{h}h {m:02}min {s:02}s")
        } else {
            write!(f, "{m}min {s:02}s")
        }
    }
}
