//! Esquema do config.toml — versão 3.
//!
//! Campos desconhecidos são preservados via `#[serde(flatten)]` + `extra`.
//! Valores fora de faixa são corrigidos com aviso.

use serde::{Deserialize, Serialize};

/// Versão atual do esquema.
pub const SCHEMA_VERSION: u32 = 3;

// ─── Configuração raiz ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub schema_version: u32,
    pub instituicao: Instituicao,
    pub video: VideoConfig,
    pub audio: AudioConfig,
    pub gravacao: GravacaoConfig,
    pub identidade_visual: IdentidadeVisual,
    pub janela: JanelaConfig,
    pub preferencias: PreferenciasConfig,
    pub emissao: EmissaoConfig,
    /// Defensores frequentes cadastrados na agenda.
    #[serde(default)]
    pub defensores: Vec<DefensorAgenda>,
    /// Modelos de texto personalizados.
    #[serde(default)]
    pub modelos: ModelosConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            instituicao: Instituicao::default(),
            video: VideoConfig::default(),
            audio: AudioConfig::default(),
            gravacao: GravacaoConfig::default(),
            identidade_visual: IdentidadeVisual::default(),
            janela: JanelaConfig::default(),
            preferencias: PreferenciasConfig::default(),
            emissao: EmissaoConfig::default(),
            defensores: Vec::new(),
            modelos: ModelosConfig::default(),
        }
    }
}

impl AppConfig {
    /// Corrige valores fora de faixa e retorna lista de campos corrigidos.
    pub fn sanitize(&mut self) -> Vec<String> {
        let mut warnings = Vec::new();

        // Transparência da tarja: 0–100
        let t = &mut self.video.tarja.transparencia_fundo_pct;
        if *t > 100 {
            warnings.push(format!("video.tarja.transparencia_fundo_pct={t} → corrigido para 100"));
            *t = 100;
        }

        // Bitrate de vídeo: 500–50000 kbps
        let vbr = &mut self.gravacao.video_bitrate_kbps;
        if *vbr < 500 {
            warnings.push(format!("gravacao.video_bitrate_kbps={vbr} → corrigido para 500"));
            *vbr = 500;
        } else if *vbr > 50_000 {
            warnings.push(format!("gravacao.video_bitrate_kbps={vbr} → corrigido para 50000"));
            *vbr = 50_000;
        }

        // Alerta de espaço: mínimo razoável
        if self.gravacao.alerta_espaco_min_minutos < 5 {
            warnings.push("gravacao.alerta_espaco_min_minutos < 5 → corrigido para 5".to_string());
            self.gravacao.alerta_espaco_min_minutos = 5;
        }

        warnings
    }
}

// ─── Instituição ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Instituicao {
    pub orgao: String,
    pub autoridade_titulo: String,
    pub autoridade_nome: String,
    pub responsavel_gravacao: String,
}

impl Default for Instituicao {
    fn default() -> Self {
        Self {
            orgao: "Ministério Público do Estado do Tocantins".to_string(),
            autoridade_titulo: "Promotor de Justiça".to_string(),
            autoridade_nome: String::new(),
            responsavel_gravacao: String::new(),
        }
    }
}

// ─── Vídeo ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VideoConfig {
    pub tarja: TarjaConfig,
    pub layout_padrao: Layout,
    pub foco_automatico: bool,
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            tarja: TarjaConfig::default(),
            layout_padrao: Layout::LadoALado,
            foco_automatico: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Layout {
    LadoALado,
    PipDepoente,
    PipMesa,
    Camera1Unica,
    Camera2Unica,
}

impl Default for Layout {
    fn default() -> Self { Self::LadoALado }
}

impl Layout {
    pub fn label(&self) -> &str {
        match self {
            Self::LadoALado => "Lado a lado",
            Self::PipDepoente => "PiP — depoente em destaque",
            Self::PipMesa => "PiP — mesa em destaque",
            Self::Camera1Unica => "Câmera 1 única",
            Self::Camera2Unica => "Câmera 2 única",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::LadoALado,
            Self::PipDepoente,
            Self::PipMesa,
            Self::Camera1Unica,
            Self::Camera2Unica,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TarjaConfig {
    /// Transparência do fundo (0 = opaco, 100 = sem fundo).
    pub transparencia_fundo_pct: u8,
    pub cor_fundo: CorTarja,
    pub altura: AlturaTarja,
    pub posicao: PosicaoTarja,
    pub logo_na_tarja: bool,
    pub mostrar_defesa: bool,
}

impl Default for TarjaConfig {
    fn default() -> Self {
        Self {
            transparencia_fundo_pct: 6, // 6% = alpha 0.94 (padrão da versão web)
            cor_fundo: CorTarja::Preto,
            altura: AlturaTarja::Normal,
            posicao: PosicaoTarja::Inferior,
            logo_na_tarja: false,
            mostrar_defesa: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CorTarja {
    Preto,
    AzulInstitucional,
    CinzaGrafite,
}

impl CorTarja {
    /// Cor RGBA em [0, 255].
    pub fn rgba(&self) -> [u8; 4] {
        match self {
            Self::Preto => [0, 0, 0, 255],
            Self::AzulInstitucional => [0, 44, 95, 255],
            Self::CinzaGrafite => [45, 45, 45, 255],
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Preto => "Preto",
            Self::AzulInstitucional => "Azul institucional",
            Self::CinzaGrafite => "Cinza grafite",
        }
    }
}

impl Default for CorTarja {
    fn default() -> Self { Self::Preto }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AlturaTarja {
    Compacta,
    Normal,
    Ampla,
}

impl AlturaTarja {
    /// Altura em pixels (na composição 1920×1080).
    pub fn pixels(&self) -> u32 {
        match self {
            Self::Compacta => 54,
            Self::Normal => 72,
            Self::Ampla => 96,
        }
    }
    pub fn label(&self) -> &str {
        match self {
            Self::Compacta => "Compacta",
            Self::Normal => "Normal",
            Self::Ampla => "Ampla",
        }
    }
}

impl Default for AlturaTarja {
    fn default() -> Self { Self::Normal }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PosicaoTarja {
    Inferior,
    Superior,
}

impl Default for PosicaoTarja {
    fn default() -> Self { Self::Inferior }
}

// ─── Áudio ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    pub modo: ModoAudio,
    pub mic1_rotulo: String,
    pub mic2_rotulo: String,
    pub mic2_ativo_por_padrao: bool,
    pub mic1_ganho_db: f32,
    pub mic2_ganho_db: f32,
    /// ID estável do SO para mic 1 (lembrado entre sessões).
    pub mic1_device_id: Option<String>,
    /// ID estável do SO para mic 2.
    pub mic2_device_id: Option<String>,
    /// ID estável do SO para câmera 1.
    pub cam1_device_id: Option<String>,
    /// ID estável do SO para câmera 2.
    pub cam2_device_id: Option<String>,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            modo: ModoAudio::FaixasSeparadas,
            mic1_rotulo: "Autoridade/Mesa".to_string(),
            mic2_rotulo: "Depoente".to_string(),
            mic2_ativo_por_padrao: false,
            mic1_ganho_db: 0.0,
            mic2_ganho_db: 0.0,
            mic1_device_id: None,
            mic2_device_id: None,
            cam1_device_id: None,
            cam2_device_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModoAudio {
    /// Dois microfones somados em uma única faixa de áudio.
    MixagemUnica,
    /// Três faixas: mix, mic1, mic2.
    FaixasSeparadas,
}

impl Default for ModoAudio {
    fn default() -> Self { Self::FaixasSeparadas }
}

impl ModoAudio {
    pub fn label(&self) -> &str {
        match self {
            Self::MixagemUnica => "Mixagem única",
            Self::FaixasSeparadas => "Faixas separadas + mixagem",
        }
    }
}

// ─── Gravação ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GravacaoConfig {
    pub pasta_destino: Option<std::path::PathBuf>,
    pub video_bitrate_kbps: u32,
    pub audio_bitrate_kbps: u32,
    /// Alertar quando espaço livre for suficiente para menos de N minutos.
    pub alerta_espaco_min_minutos: u32,
    /// Alertar quando espaço livre for menor que N bytes.
    pub alerta_espaco_min_bytes: u64,
    /// Tempo (segundos) sem sinal antes de alertar sobre microfone silencioso.
    pub alerta_silencio_secs: u32,
}

impl Default for GravacaoConfig {
    fn default() -> Self {
        Self {
            pasta_destino: None,
            video_bitrate_kbps: 4000,
            audio_bitrate_kbps: 128,
            alerta_espaco_min_minutos: 30,
            alerta_espaco_min_bytes: 2 * 1024 * 1024 * 1024, // 2 GB
            alerta_silencio_secs: 10,
        }
    }
}

// ─── Identidade visual ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct IdentidadeVisual {
    /// Caminho personalizado para o cabeçalho dos documentos (None = padrão embutido).
    pub cabecalho_path: Option<std::path::PathBuf>,
    /// Largura do cabeçalho como % da largura útil (50–100).
    pub cabecalho_largura_pct: Option<u8>,
    pub cabecalho_em_todas_paginas: bool,
    pub cabecalho_ativo: bool,
    /// Caminho personalizado para a logomarca na interface (None = icon.ico padrão).
    pub logomarca_interface_path: Option<std::path::PathBuf>,
    /// Caminho personalizado para a logomarca na tarja (None = desligado).
    pub logomarca_tarja_path: Option<std::path::PathBuf>,
}

impl IdentidadeVisual {
    pub fn cabecalho_largura_pct_valida(&self) -> u8 {
        self.cabecalho_largura_pct
            .map(|v| v.clamp(50, 100))
            .unwrap_or(100)
    }
}

// ─── Janela ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct JanelaConfig {
    pub pos_x: Option<f32>,
    pub pos_y: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub maximized: bool,
}

// ─── Preferências ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PreferenciasConfig {
    pub escala_dpi: Option<f32>,
    pub tamanho_fonte_base: Option<f32>,
    pub primeiro_uso_concluido: bool,
    pub cidade_emissao: String,
}

impl Default for PreferenciasConfig {
    fn default() -> Self {
        Self {
            escala_dpi: None,
            tamanho_fonte_base: None,
            primeiro_uso_concluido: false,
            cidade_emissao: "Palmas".to_string(),
        }
    }
}

// ─── Emissão ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EmissaoConfig {
    pub local: String,
}

impl Default for EmissaoConfig {
    fn default() -> Self {
        Self { local: "Palmas".to_string() }
    }
}

// ─── Agenda de defensores ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefensorAgenda {
    pub nome: String,
    #[serde(default)]
    pub tipo: clio_core::models::TipoDefesa,
    pub oab: Option<String>,
}

// ─── Modelos de texto ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ModelosConfig {
    /// None = usar padrão embutido.
    pub extrato_template: Option<String>,
    pub relatorio_template: Option<String>,
    /// Histórico das últimas 10 versões do extrato.
    #[serde(default)]
    pub extrato_historico: Vec<String>,
    /// Histórico das últimas 10 versões do relatório.
    #[serde(default)]
    pub relatorio_historico: Vec<String>,
}

impl ModelosConfig {
    pub const MAX_HISTORY: usize = 10;

    pub fn salvar_extrato(&mut self, novo: String) {
        if let Some(atual) = &self.extrato_template {
            self.extrato_historico.push(atual.clone());
            if self.extrato_historico.len() > Self::MAX_HISTORY {
                self.extrato_historico.remove(0);
            }
        }
        self.extrato_template = Some(novo);
    }

    pub fn salvar_relatorio(&mut self, novo: String) {
        if let Some(atual) = &self.relatorio_template {
            self.relatorio_historico.push(atual.clone());
            if self.relatorio_historico.len() > Self::MAX_HISTORY {
                self.relatorio_historico.remove(0);
            }
        }
        self.relatorio_template = Some(novo);
    }
}
