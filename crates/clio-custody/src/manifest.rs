//! Manifesto JSON da gravação — gerado ao encerrar a sessão e selado.

use serde::{Deserialize, Serialize};
use crate::chain::ChainBlock;
use crate::error::CustodyResult;
use clio_core::models::{Participante, Defesa, EventoSessao};
use clio_core::types::{PipelineParams, Timestamp};

/// Manifesto completo da sessão de gravação.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub session_id: String,
    pub app_version: String,
    pub app_build: String,
    pub depoente_nome: String,
    pub depoente_qualificacao: String,
    pub processo_numero: String,
    pub processo_tipo: String,
    pub orgao: String,
    pub autoridade_titulo: String,
    pub autoridade_nome: String,
    pub responsavel_gravacao: String,
    pub participantes: Vec<ParticipanteManifest>,
    pub defesas: Vec<DefesaManifest>,
    pub inicio_iso8601: String,
    pub fim_iso8601: String,
    pub duracao_liquida_secs: u64,
    pub tempo_pausado_secs: u64,
    pub num_pausas: u32,
    pub recuperada: bool,
    pub pipeline: PipelineManifest,
    pub arquivo_nome: String,
    pub arquivo_tamanho_bytes: u64,
    pub arquivo_sha256: String,
    pub cadeia_hashes: Vec<ChainBlock>,
    pub eventos: Vec<EventoManifest>,
    /// Assinatura Ed25519 do manifesto (hex), preenchida após `seal::assinar`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assinatura_ed25519: Option<String>,
    /// Impressão digital da chave pública (hex dos primeiros 8 bytes), se selado.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selo_fingerprint: Option<String>,
}

pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipanteManifest {
    pub nome: String,
    pub papel: String,
    pub documento: Option<String>,
    pub presenca: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefesaManifest {
    pub nome: String,
    pub tipo: String,
    pub oab: Option<String>,
    pub representa: Option<String>,
    pub presenca: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineManifest {
    pub resolucao: String,
    pub fps: u32,
    pub codec_video: String,
    pub codec_audio: String,
    pub container: String,
    pub bitrate_video_kbps: u32,
    pub bitrate_audio_kbps: u32,
    pub modo_audio: String,
    pub microfones: Vec<MicrofoneManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrofoneManifest {
    pub rotulo: String,
    pub device_id: String,
    pub ganho_db: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventoManifest {
    pub hora_iso8601: String,
    pub tipo: String,
    pub descricao: String,
}

/// Parâmetros para geração do manifesto.
pub struct ManifestParams<'a> {
    pub session_id: &'a str,
    pub app_version: &'a str,
    pub app_build: &'a str,
    pub depoente_nome: &'a str,
    pub depoente_qualificacao: &'a str,
    pub processo_numero: &'a str,
    pub processo_tipo: &'a str,
    pub orgao: &'a str,
    pub autoridade_titulo: &'a str,
    pub autoridade_nome: &'a str,
    pub responsavel_gravacao: &'a str,
    pub participantes: &'a [Participante],
    pub defesas: &'a [Defesa],
    pub inicio: &'a Timestamp,
    pub fim: &'a Timestamp,
    pub duracao_liquida_secs: u64,
    pub tempo_pausado_secs: u64,
    pub num_pausas: u32,
    pub recuperada: bool,
    pub pipeline: &'a PipelineParams,
    pub modo_audio: &'a str,
    pub microfones: Vec<MicrofoneManifest>,
    pub arquivo_nome: &'a str,
    pub arquivo_tamanho_bytes: u64,
    pub arquivo_sha256: &'a str,
    pub cadeia_hashes: Vec<ChainBlock>,
    pub eventos: &'a [EventoSessao],
}

/// Gera o manifesto JSON a partir dos parâmetros da sessão.
pub fn gerar_manifesto(p: ManifestParams<'_>) -> Manifest {
    use time::format_description::well_known::Rfc3339;

    let inicio_str = p.inicio.format(&Rfc3339).unwrap_or_default();
    let fim_str = p.fim.format(&Rfc3339).unwrap_or_default();

    let participantes = p
        .participantes
        .iter()
        .map(|pt| ParticipanteManifest {
            nome: pt.nome.clone(),
            papel: pt.papel.label().to_string(),
            documento: pt.documento.clone(),
            presenca: pt.presenca.label().to_string(),
        })
        .collect();

    let defesas = p
        .defesas
        .iter()
        .map(|d| DefesaManifest {
            nome: d.nome.clone(),
            tipo: d.tipo.label().to_string(),
            oab: d.oab.clone(),
            representa: d.representa.clone(),
            presenca: d.presenca.label().to_string(),
        })
        .collect();

    let eventos = p
        .eventos
        .iter()
        .map(|e| EventoManifest {
            hora_iso8601: e.hora.format(&Rfc3339).unwrap_or_default(),
            tipo: e.tipo.label().to_string(),
            descricao: e.descricao.clone(),
        })
        .collect();

    Manifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        session_id: p.session_id.to_string(),
        app_version: p.app_version.to_string(),
        app_build: p.app_build.to_string(),
        depoente_nome: p.depoente_nome.to_string(),
        depoente_qualificacao: p.depoente_qualificacao.to_string(),
        processo_numero: p.processo_numero.to_string(),
        processo_tipo: p.processo_tipo.to_string(),
        orgao: p.orgao.to_string(),
        autoridade_titulo: p.autoridade_titulo.to_string(),
        autoridade_nome: p.autoridade_nome.to_string(),
        responsavel_gravacao: p.responsavel_gravacao.to_string(),
        participantes,
        defesas,
        inicio_iso8601: inicio_str,
        fim_iso8601: fim_str,
        duracao_liquida_secs: p.duracao_liquida_secs,
        tempo_pausado_secs: p.tempo_pausado_secs,
        num_pausas: p.num_pausas,
        recuperada: p.recuperada,
        pipeline: PipelineManifest {
            resolucao: p.pipeline.resolution.to_string(),
            fps: p.pipeline.fps,
            codec_video: p.pipeline.video_codec.to_string(),
            codec_audio: p.pipeline.audio_codec.to_string(),
            container: p.pipeline.container.to_string(),
            bitrate_video_kbps: p.pipeline.video_bitrate_kbps,
            bitrate_audio_kbps: p.pipeline.audio_bitrate_kbps,
            modo_audio: p.modo_audio.to_string(),
            microfones: p.microfones,
        },
        arquivo_nome: p.arquivo_nome.to_string(),
        arquivo_tamanho_bytes: p.arquivo_tamanho_bytes,
        arquivo_sha256: p.arquivo_sha256.to_string(),
        cadeia_hashes: p.cadeia_hashes,
        eventos,
        assinatura_ed25519: None,
        selo_fingerprint: None,
    }
}

impl Manifest {
    /// Serializa para JSON com indentação.
    pub fn to_json_pretty(&self) -> CustodyResult<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Salva o manifesto em disco como `<nome_base>.manifest.json`.
    pub fn salvar(&self, path: &std::path::Path) -> CustodyResult<()> {
        use std::io::Write;
        let json = self.to_json_pretty()?;
        let mut f = std::fs::File::create(path)?;
        f.write_all(json.as_bytes())?;
        f.sync_all()?;
        Ok(())
    }

    /// Serializa o manifesto **sem** assinatura para assinar (campo `assinatura_ed25519` = None).
    pub fn bytes_para_assinar(&self) -> CustodyResult<Vec<u8>> {
        let mut m = self.clone();
        m.assinatura_ed25519 = None;
        m.selo_fingerprint = None;
        Ok(serde_json::to_vec_pretty(&m)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clio_core::types::{PipelineParams, Resolution, VideoCodec, AudioCodec, Container};
    use time::OffsetDateTime;

    fn params_basicos() -> Manifest {
        let pipeline = PipelineParams {
            resolution: Resolution::FULL_HD,
            fps: 30,
            video_codec: VideoCodec::H264,
            audio_codec: AudioCodec::Aac,
            container: Container::Mkv,
            video_bitrate_kbps: 4000,
            audio_bitrate_kbps: 128,
        };
        let now = OffsetDateTime::now_utc();
        gerar_manifesto(ManifestParams {
            session_id: "test-123",
            app_version: "0.1.0",
            app_build: "abc123",
            depoente_nome: "João da Silva",
            depoente_qualificacao: "Testemunha",
            processo_numero: "0001234-56.2026.8.27.2729",
            processo_tipo: "Autos",
            orgao: "MPTO/GAECO",
            autoridade_titulo: "Promotor de Justiça",
            autoridade_nome: "Dr. Teste",
            responsavel_gravacao: "Secretário Teste",
            participantes: &[],
            defesas: &[],
            inicio: &now,
            fim: &now,
            duracao_liquida_secs: 3600,
            tempo_pausado_secs: 0,
            num_pausas: 0,
            recuperada: false,
            pipeline: &pipeline,
            modo_audio: "Faixas separadas",
            microfones: vec![],
            arquivo_nome: "Oitiva_Joao_20261117_14h30.mkv",
            arquivo_tamanho_bytes: 1_500_000_000,
            arquivo_sha256: "aabbcc",
            cadeia_hashes: vec![],
            eventos: &[],
        })
    }

    #[test]
    fn manifesto_serializa_e_desserializa() {
        let m = params_basicos();
        let json = m.to_json_pretty().unwrap();
        let m2: Manifest = serde_json::from_str(&json).unwrap();
        assert_eq!(m.session_id, m2.session_id);
        assert_eq!(m.depoente_nome, m2.depoente_nome);
    }

    #[test]
    fn bytes_para_assinar_sem_assinatura() {
        let mut m = params_basicos();
        m.assinatura_ed25519 = Some("FAKE_SIGNATURE".to_string());
        let bytes = m.bytes_para_assinar().unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(parsed.get("assinatura_ed25519").is_none());
    }
}
