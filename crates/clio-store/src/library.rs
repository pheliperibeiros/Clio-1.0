//! Biblioteca de gravações concluídas.

use crate::db::Database;
use crate::error::StoreResult;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusGravacao {
    Concluida,
    Recuperada,
    Exportada,
}

impl StatusGravacao {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Concluida => "concluida",
            Self::Recuperada => "recuperada",
            Self::Exportada => "exportada",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "recuperada" => Self::Recuperada,
            "exportada" => Self::Exportada,
            _ => Self::Concluida,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Concluida => "Concluída",
            Self::Recuperada => "Recuperada",
            Self::Exportada => "Exportada",
        }
    }
}

/// Entrada da biblioteca para uma gravação.
#[derive(Debug, Clone)]
pub struct GravacaoInfo {
    pub id: String,
    pub depoente_nome: String,
    pub depoente_qualificacao: String,
    pub processo_numero: String,
    pub orgao: String,
    pub inicio_iso8601: String,
    pub fim_iso8601: Option<String>,
    pub duracao_liquida_secs: Option<u64>,
    pub arquivo_path: String,
    pub arquivo_tamanho_bytes: Option<u64>,
    pub arquivo_sha256: Option<String>,
    pub status: StatusGravacao,
    pub recuperada: bool,
    pub manifesto_path: Option<String>,
}

impl Database {
    /// Registra uma gravação concluída na biblioteca.
    pub fn registrar_gravacao(&self, info: &GravacaoInfo) -> StoreResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO gravacoes (
                id, depoente_nome, depoente_qualificacao, processo_numero, orgao,
                inicio_iso8601, fim_iso8601, duracao_liquida_secs,
                arquivo_path, arquivo_tamanho_bytes, arquivo_sha256,
                status, recuperada, manifesto_path
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                info.id,
                info.depoente_nome,
                info.depoente_qualificacao,
                info.processo_numero,
                info.orgao,
                info.inicio_iso8601,
                info.fim_iso8601,
                info.duracao_liquida_secs.map(|v| v as i64),
                info.arquivo_path,
                info.arquivo_tamanho_bytes.map(|v| v as i64),
                info.arquivo_sha256,
                info.status.as_str(),
                info.recuperada as i32,
                info.manifesto_path,
            ],
        )?;
        debug!("Gravação registrada na biblioteca: {}", info.id);
        Ok(())
    }

    /// Lista todas as gravações, ordenadas por início (mais recente primeiro).
    pub fn listar_gravacoes(&self, filtro: Option<&str>) -> StoreResult<Vec<GravacaoInfo>> {
        let sql = if filtro.is_some() {
            "SELECT id, depoente_nome, depoente_qualificacao, processo_numero, orgao,
                    inicio_iso8601, fim_iso8601, duracao_liquida_secs,
                    arquivo_path, arquivo_tamanho_bytes, arquivo_sha256,
                    status, recuperada, manifesto_path
             FROM gravacoes
             WHERE depoente_nome LIKE ?1 OR processo_numero LIKE ?1
             ORDER BY inicio_iso8601 DESC"
        } else {
            "SELECT id, depoente_nome, depoente_qualificacao, processo_numero, orgao,
                    inicio_iso8601, fim_iso8601, duracao_liquida_secs,
                    arquivo_path, arquivo_tamanho_bytes, arquivo_sha256,
                    status, recuperada, manifesto_path
             FROM gravacoes
             ORDER BY inicio_iso8601 DESC"
        };

        let mut stmt = self.conn.prepare(sql)?;
        let like_pattern = filtro.map(|f| format!("%{f}%"));

        let rows = if let Some(ref pattern) = like_pattern {
            stmt.query_map(params![pattern], Self::row_to_gravacao)?
        } else {
            stmt.query_map([], Self::row_to_gravacao)?
        };

        let mut result = Vec::new();
        for row in rows {
            match row {
                Ok(g) => result.push(g),
                Err(e) => warn!("Erro ao ler gravação: {e}"),
            }
        }
        Ok(result)
    }

    fn row_to_gravacao(row: &rusqlite::Row<'_>) -> rusqlite::Result<GravacaoInfo> {
        Ok(GravacaoInfo {
            id: row.get(0)?,
            depoente_nome: row.get(1)?,
            depoente_qualificacao: row.get(2)?,
            processo_numero: row.get(3)?,
            orgao: row.get(4)?,
            inicio_iso8601: row.get(5)?,
            fim_iso8601: row.get(6)?,
            duracao_liquida_secs: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
            arquivo_path: row.get(8)?,
            arquivo_tamanho_bytes: row.get::<_, Option<i64>>(9)?.map(|v| v as u64),
            arquivo_sha256: row.get(10)?,
            status: StatusGravacao::from_str(&row.get::<_, String>(11)?),
            recuperada: row.get::<_, i32>(12)? != 0,
            manifesto_path: row.get(13)?,
        })
    }

    /// Remove uma gravação do banco (não apaga o arquivo — isso é feito pela UI).
    pub fn remover_gravacao(&self, id: &str) -> StoreResult<()> {
        self.conn
            .execute("DELETE FROM gravacoes WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Atualiza o status de uma gravação.
    pub fn atualizar_status(&self, id: &str, status: StatusGravacao) -> StoreResult<()> {
        self.conn.execute(
            "UPDATE gravacoes SET status = ?1 WHERE id = ?2",
            params![status.as_str(), id],
        )?;
        Ok(())
    }

    /// Registra uma exportação de documento.
    pub fn registrar_exportacao(
        &self,
        gravacao_id: &str,
        formato: &str,
        tipo: &str,
        arquivo_path: &str,
        sha256: Option<&str>,
    ) -> StoreResult<()> {
        self.conn.execute(
            "INSERT INTO exportacoes (gravacao_id, formato, tipo, arquivo_path, arquivo_sha256)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![gravacao_id, formato, tipo, arquivo_path, sha256],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registrar_e_listar_gravacao() {
        let db = Database::open_in_memory().unwrap();
        let info = GravacaoInfo {
            id: "abc-123".to_string(),
            depoente_nome: "Maria Silva".to_string(),
            depoente_qualificacao: "Testemunha".to_string(),
            processo_numero: "0001234".to_string(),
            orgao: "MPTO".to_string(),
            inicio_iso8601: "2026-11-17T14:00:00Z".to_string(),
            fim_iso8601: Some("2026-11-17T15:30:00Z".to_string()),
            duracao_liquida_secs: Some(5400),
            arquivo_path: "/gravacoes/maria.mkv".to_string(),
            arquivo_tamanho_bytes: Some(1_500_000_000),
            arquivo_sha256: Some("aabbcc".to_string()),
            status: StatusGravacao::Concluida,
            recuperada: false,
            manifesto_path: Some("/gravacoes/maria.manifest.json".to_string()),
        };
        db.registrar_gravacao(&info).unwrap();

        let lista = db.listar_gravacoes(None).unwrap();
        assert_eq!(lista.len(), 1);
        assert_eq!(lista[0].depoente_nome, "Maria Silva");
    }

    #[test]
    fn filtro_por_nome() {
        let db = Database::open_in_memory().unwrap();
        for nome in &["Ana Santos", "Pedro Lima", "Ana Costa"] {
            let info = GravacaoInfo {
                id: uuid::Uuid::new_v4().to_string(),
                depoente_nome: nome.to_string(),
                depoente_qualificacao: "Testemunha".to_string(),
                processo_numero: "0001".to_string(),
                orgao: "MPTO".to_string(),
                inicio_iso8601: "2026-11-17T14:00:00Z".to_string(),
                fim_iso8601: None,
                duracao_liquida_secs: None,
                arquivo_path: format!("/gravacoes/{nome}.mkv"),
                arquivo_tamanho_bytes: None,
                arquivo_sha256: None,
                status: StatusGravacao::Concluida,
                recuperada: false,
                manifesto_path: None,
            };
            db.registrar_gravacao(&info).unwrap();
        }

        let resultado = db.listar_gravacoes(Some("Ana")).unwrap();
        assert_eq!(resultado.len(), 2);
    }
}
