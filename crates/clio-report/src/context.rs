//! Contexto de variáveis passado aos modelos Jinja2.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Contexto completo passado ao motor de templates.
/// Todos os campos correspondem às variáveis documentadas na especificação.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportContext {
    pub orgao: String,
    pub processo: ProcessoCtx,
    pub depoente: DepoenteCtx,
    pub autoridade: AutoridadeCtx,
    pub responsavel_gravacao: String,
    pub participantes: Vec<ParticipanteCtx>,
    pub defesas: Vec<DefesaCtx>,
    pub sessao: SessaoCtx,
    pub arquivo: ArquivoCtx,
    pub video: VideoCtx,
    pub audio: AudioCtx,
    pub eventos: Vec<EventoCtx>,
    pub app: AppCtx,
    pub ambiente: AmbienteCtx,
    pub emissao: EmissaoCtx,
    pub selo: SeloCtx,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessoCtx {
    pub numero: String,
    pub tipo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepoenteCtx {
    pub nome: String,
    pub qualificacao: String,
    pub documento: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoridadeCtx {
    pub titulo: String,
    pub nome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipanteCtx {
    pub nome: String,
    pub papel: String,
    pub documento: Option<String>,
    pub presenca: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefesaCtx {
    pub nome: String,
    pub tipo: String,
    pub oab: Option<String>,
    pub representa: Option<String>,
    pub presenca: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessaoCtx {
    /// ISO-8601 do início (filtros `data`, `hora`, `data_longa` convertem).
    pub inicio: String,
    pub fim: String,
    pub duracao_liquida: u64,
    pub tempo_pausado: u64,
    pub num_pausas: u32,
    pub recuperada: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArquivoCtx {
    pub nome: String,
    pub tamanho: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoCtx {
    pub resolucao: String,
    pub fps: u32,
    pub codec: String,
    pub container: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioCtx {
    pub modo: String,
    pub microfones: Vec<MicrofoneCtx>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrofoneCtx {
    pub rotulo: String,
    pub dispositivo: String,
    pub ganho_db: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventoCtx {
    pub hora: String,
    pub tipo: String,
    pub descricao: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppCtx {
    pub versao: String,
    pub build: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmbienteCtx {
    pub so: String,
    pub maquina: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissaoCtx {
    pub data: String,
    pub local: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeloCtx {
    pub fingerprint: Option<String>,
}

impl ReportContext {
    /// Converte para `serde_json::Value` para ser usado pelo motor de templates.
    pub fn to_value(&self) -> serde_json::Result<Value> {
        serde_json::to_value(self)
    }

    /// Contexto de exemplo para prévia ao vivo no editor de modelos.
    pub fn exemplo() -> Self {
        Self {
            orgao: "Ministério Público do Estado do Tocantins — GAECO".to_string(),
            processo: ProcessoCtx {
                numero: "0001234-56.2026.8.27.2729".to_string(),
                tipo: "Autos".to_string(),
            },
            depoente: DepoenteCtx {
                nome: "João da Silva".to_string(),
                qualificacao: "Testemunha".to_string(),
                documento: Some("CPF 123.456.789-00".to_string()),
            },
            autoridade: AutoridadeCtx {
                titulo: "Promotor de Justiça".to_string(),
                nome: "Dr. Exemplo da Silva".to_string(),
            },
            responsavel_gravacao: "Secretária Maria Santos".to_string(),
            participantes: vec![
                ParticipanteCtx {
                    nome: "Dr. Exemplo da Silva".to_string(),
                    papel: "Autoridade condutora".to_string(),
                    documento: None,
                    presenca: "Presencial".to_string(),
                },
                ParticipanteCtx {
                    nome: "Dra. Advogada Exemplo".to_string(),
                    papel: "Defesa".to_string(),
                    documento: Some("OAB/TO 1234".to_string()),
                    presenca: "Presencial".to_string(),
                },
            ],
            defesas: vec![DefesaCtx {
                nome: "Dra. Advogada Exemplo".to_string(),
                tipo: "Advogada constituída".to_string(),
                oab: Some("OAB/TO 1234".to_string()),
                representa: Some("João da Silva".to_string()),
                presenca: "Presencial".to_string(),
            }],
            sessao: SessaoCtx {
                inicio: "2026-11-17T14:00:00-03:00".to_string(),
                fim: "2026-11-17T15:30:00-03:00".to_string(),
                duracao_liquida: 5100,
                tempo_pausado: 300,
                num_pausas: 1,
                recuperada: false,
            },
            arquivo: ArquivoCtx {
                nome: "Oitiva_Joao_da_Silva_Proc-0001234_20261117_14h00.mkv".to_string(),
                tamanho: 1_572_864_000,
                sha256: "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f9".to_string(),
            },
            video: VideoCtx {
                resolucao: "1920×1080".to_string(),
                fps: 30,
                codec: "H.264".to_string(),
                container: "Matroska (MKV)".to_string(),
            },
            audio: AudioCtx {
                modo: "Faixas separadas + mixagem".to_string(),
                microfones: vec![
                    MicrofoneCtx {
                        rotulo: "Autoridade/Mesa".to_string(),
                        dispositivo: "Microfone USB (Realtek)".to_string(),
                        ganho_db: 0.0,
                    },
                    MicrofoneCtx {
                        rotulo: "Depoente".to_string(),
                        dispositivo: "Microfone de Lapela USB".to_string(),
                        ganho_db: 3.0,
                    },
                ],
            },
            eventos: vec![
                EventoCtx {
                    hora: "2026-11-17T14:00:00-03:00".to_string(),
                    tipo: "Início da gravação".to_string(),
                    descricao: "Gravação iniciada".to_string(),
                },
                EventoCtx {
                    hora: "2026-11-17T14:45:00-03:00".to_string(),
                    tipo: "Pausa (recesso)".to_string(),
                    descricao: "Recesso solicitado pela defesa".to_string(),
                },
                EventoCtx {
                    hora: "2026-11-17T14:50:00-03:00".to_string(),
                    tipo: "Retomada".to_string(),
                    descricao: "Oitiva retomada".to_string(),
                },
            ],
            app: AppCtx {
                versao: "0.1.0".to_string(),
                build: "abc1234def5678".to_string(),
            },
            ambiente: AmbienteCtx {
                so: "Windows 11 Pro 24H2".to_string(),
                maquina: "MPTO-GAECO-01".to_string(),
            },
            emissao: EmissaoCtx {
                data: "2026-11-17T15:35:00-03:00".to_string(),
                local: "Palmas".to_string(),
            },
            selo: SeloCtx {
                fingerprint: Some("a1b2c3d4e5f60718".to_string()),
            },
        }
    }
}
