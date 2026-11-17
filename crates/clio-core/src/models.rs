//! Modelos de domínio: participantes, depoentes, sessões.

use crate::types::{DurationSecs, ParticipantId, PipelineParams, SessionId, Timestamp};
use serde::{Deserialize, Serialize};

// ─── Depoente ────────────────────────────────────────────────────────────────

/// Papel do depoente na sessão.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Qualificacao {
    #[default]
    Testemunha,
    Vitima,
    Investigado,
    Reu,
    Perito,
    Outro(String),
}

impl Qualificacao {
    pub fn label(&self) -> &str {
        match self {
            Self::Testemunha => "Testemunha",
            Self::Vitima => "Vítima",
            Self::Investigado => "Investigado",
            Self::Reu => "Réu",
            Self::Perito => "Perito",
            Self::Outro(s) => s.as_str(),
        }
    }

    pub fn all_standard() -> &'static [&'static str] {
        &["Testemunha", "Vítima", "Investigado", "Réu", "Perito"]
    }
}

/// Pessoa sendo ouvida.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Depoente {
    pub nome: String,
    pub qualificacao: Qualificacao,
    /// CPF, RG ou outro identificador (opcional).
    pub documento: Option<String>,
}

// ─── Processo ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TipoProcesso {
    #[default]
    Autos,
    Inquerito,
    ProcedimentoInvestigatorio,
    Outro(String),
}

impl TipoProcesso {
    pub fn label(&self) -> &str {
        match self {
            Self::Autos => "Autos",
            Self::Inquerito => "Inquérito",
            Self::ProcedimentoInvestigatorio => "Procedimento Investigatório",
            Self::Outro(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Processo {
    pub numero: String,
    pub tipo: TipoProcesso,
}

// ─── Participantes ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Papel {
    AutoridadeCondutora,
    MembroMp,
    Defesa,
    AssistenteAcusacao,
    Interprete,
    EscrivaoSecretario,
    Outro(String),
}

impl Papel {
    pub fn label(&self) -> &str {
        match self {
            Self::AutoridadeCondutora => "Autoridade condutora",
            Self::MembroMp => "Membro do MP",
            Self::Defesa => "Defesa",
            Self::AssistenteAcusacao => "Assistente de acusação",
            Self::Interprete => "Intérprete",
            Self::EscrivaoSecretario => "Escrivão/Secretário",
            Self::Outro(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Presenca {
    Presencial,
    Remota,
}

impl Presenca {
    pub fn label(&self) -> &str {
        match self {
            Self::Presencial => "Presencial",
            Self::Remota => "Remota",
        }
    }
}

impl Default for Presenca {
    fn default() -> Self {
        Self::Presencial
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participante {
    pub id: ParticipantId,
    pub nome: String,
    pub papel: Papel,
    pub documento: Option<String>,
    pub presenca: Presenca,
}

impl Participante {
    pub fn new(nome: String, papel: Papel) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            nome,
            papel,
            documento: None,
            presenca: Presenca::default(),
        }
    }
}

// ─── Defesa ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipoDefesa {
    AdvogadoConstituido,
    DefensorPublico,
    AdvogadoDativo,
}

impl TipoDefesa {
    pub fn label(&self) -> &str {
        match self {
            Self::AdvogadoConstituido => "Advogado constituído",
            Self::DefensorPublico => "Defensor público",
            Self::AdvogadoDativo => "Advogado dativo",
        }
    }
}

impl Default for TipoDefesa {
    fn default() -> Self {
        Self::AdvogadoConstituido
    }
}

/// Defensor presente na sessão.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defesa {
    pub id: ParticipantId,
    pub nome: String,
    pub tipo: TipoDefesa,
    /// OAB/UF no formato "OAB/TO 1234" (opcional — não bloqueia se ausente).
    pub oab: Option<String>,
    /// Nome de quem representa (vinculado ao depoente ou nome livre).
    pub representa: Option<String>,
    pub presenca: Presenca,
}

impl Defesa {
    pub fn new(nome: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            nome,
            tipo: TipoDefesa::default(),
            oab: None,
            representa: None,
            presenca: Presenca::default(),
        }
    }

    /// Formata o bloco compacto para a tarja: "Nome (OAB/TO 1234)".
    pub fn tarja_label(&self) -> String {
        if let Some(oab) = &self.oab {
            format!("{} ({})", self.nome, oab)
        } else {
            self.nome.clone()
        }
    }
}

// ─── Evento de sessão ────────────────────────────────────────────────────────

/// Tipos de evento registrados no diário.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TipoEvento {
    InicioGravacao,
    Pausa,
    Retomada,
    TrocaLayout,
    TrocaMicrofone2,
    AlertaDispositivo,
    AlertaEspaco,
    TransparenciaTarja,
    MudancaTarja,
    OutroAlerta,
    EnceramentoNormal,
    EnceramentoRecuperado,
}

impl TipoEvento {
    pub fn label(&self) -> &str {
        match self {
            Self::InicioGravacao => "Início da gravação",
            Self::Pausa => "Pausa (recesso)",
            Self::Retomada => "Retomada",
            Self::TrocaLayout => "Troca de layout",
            Self::TrocaMicrofone2 => "Microfone 2 acionado/desligado",
            Self::AlertaDispositivo => "Alerta de dispositivo",
            Self::AlertaEspaco => "Alerta de espaço em disco",
            Self::TransparenciaTarja => "Transparência da tarja alterada",
            Self::MudancaTarja => "Parâmetro da tarja alterado",
            Self::OutroAlerta => "Alerta",
            Self::EnceramentoNormal => "Encerramento normal",
            Self::EnceramentoRecuperado => "Encerramento após recuperação",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventoSessao {
    pub hora: Timestamp,
    pub tipo: TipoEvento,
    pub descricao: String,
}

// ─── Sessão completa ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessaoMetadados {
    pub id: SessionId,
    pub depoente: Depoente,
    pub processo: Processo,
    pub orgao: String,
    pub autoridade_titulo: String,
    pub autoridade_nome: String,
    pub responsavel_gravacao: String,
    pub participantes: Vec<Participante>,
    pub defesas: Vec<Defesa>,
    pub inicio: Timestamp,
    pub fim: Option<Timestamp>,
    /// Duração total de pausas, em segundos.
    pub tempo_pausado_secs: u64,
    pub num_pausas: u32,
    pub recuperada: bool,
    pub pipeline: PipelineParams,
    pub caminho_arquivo: std::path::PathBuf,
    pub eventos: Vec<EventoSessao>,
}

impl SessaoMetadados {
    /// Duração líquida = (fim - início) − tempo pausado.
    pub fn duracao_liquida(&self) -> Option<DurationSecs> {
        let fim = self.fim?;
        let total = (fim - self.inicio).whole_seconds().max(0) as u64;
        Some(DurationSecs(total.saturating_sub(self.tempo_pausado_secs)))
    }

    /// Resumo da linha de defesas para a tarja.
    /// Se ≤ 3 defensores: "DEFESA: Nome (OAB) · Nome (OAB)"
    /// Se > 3: "DEFESA: N advogados"
    pub fn linha_defesa_tarja(&self) -> Option<String> {
        if self.defesas.is_empty() {
            return None;
        }
        if self.defesas.len() <= 3 {
            let partes: Vec<String> = self.defesas.iter().map(|d| d.tarja_label()).collect();
            Some(format!("DEFESA: {}", partes.join(" · ")))
        } else {
            Some(format!("DEFESA: {} advogados", self.defesas.len()))
        }
    }
}
