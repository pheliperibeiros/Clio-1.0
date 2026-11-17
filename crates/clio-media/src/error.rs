//! Erros do módulo de mídia.
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MediaError {
    #[error("GStreamer não inicializado: {0}")]
    GstInit(String),

    #[error("Elemento GStreamer não encontrado: '{0}' (plugin ausente?)")]
    ElementNotFound(String),

    #[error("Falha ao criar pipeline: {0}")]
    PipelineCreate(String),

    #[error("Falha ao iniciar pipeline: {0}")]
    PipelineStart(String),

    #[error("Dispositivo de câmera não encontrado: {0}")]
    CameraNotFound(String),

    #[error("Dispositivo de microfone não encontrado: {0}")]
    MicNotFound(String),

    #[error("Câmera '{0}' está em uso por outro aplicativo")]
    CameraInUse(String),

    #[error("Encoder H.264 não disponível (hardware ou software)")]
    EncoderUnavailable,

    #[error("Falha na tarja: {0}")]
    OverlayError(String),

    #[error("Espaço em disco insuficiente: disponível {available_gb:.1} GB")]
    InsufficientSpace { available_gb: f64 },

    #[error("Erro de I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("Canal encerrado")]
    ChannelClosed,

    #[error("Erro interno: {0}")]
    Internal(String),
}

pub type MediaResult<T> = Result<T, MediaError>;
