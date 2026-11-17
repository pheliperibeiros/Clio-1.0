//! Diário de sessão no SQLite — criado antes de iniciar o pipeline.
//!
//! Permite recuperação após crash: sessões sem `encerramento_regular = 1`
//! são detectadas na reabertura do aplicativo.

use crate::db::Database;
use crate::error::StoreResult;
use clio_core::models::EventoSessao;
use clio_custody::chain::ChainBlock;
use rusqlite::params;
use tracing::{debug, info};

/// Sessão sem encerramento regular (candidata a recuperação).
#[derive(Debug, Clone)]
pub struct SessaoPendente {
    pub id: String,
    pub depoente_nome: String,
    pub processo_numero: String,
    pub arquivo_path: String,
    pub inicio_iso8601: String,
}

/// Wrapper para operações de diário de sessão.
pub struct SessionJournal<'a> {
    pub db: &'a Database,
    pub sessao_id: String,
}

impl<'a> SessionJournal<'a> {
    /// Obtém um journal existente por ID.
    pub fn for_session(db: &'a Database, sessao_id: &str) -> Self {
        Self {
            db,
            sessao_id: sessao_id.to_string(),
        }
    }
    /// Cria um novo registro de sessão no banco, retornando o journal ativo.
    /// Deve ser chamado ANTES de iniciar o pipeline.
    pub fn iniciar(
        db: &'a Database,
        sessao_id: &str,
        depoente_nome: &str,
        processo_numero: &str,
        arquivo_path: &str,
        inicio_iso8601: &str,
        metadados_json: Option<&str>,
    ) -> StoreResult<Self> {
        db.conn.execute(
            "INSERT INTO sessoes (id, depoente_nome, processo_numero, arquivo_path, inicio_iso8601, metadados_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                sessao_id,
                depoente_nome,
                processo_numero,
                arquivo_path,
                inicio_iso8601,
                metadados_json,
            ],
        )?;
        info!("Diário de sessão iniciado: {sessao_id}");
        Ok(Self {
            db,
            sessao_id: sessao_id.to_string(),
        })
    }

    /// Registra um evento no diário.
    pub fn registrar_evento(&self, evento: &EventoSessao) -> StoreResult<()> {
        use time::format_description::well_known::Rfc3339;
        let hora_str = evento.hora.format(&Rfc3339).unwrap_or_default();
        self.db.conn.execute(
            "INSERT INTO eventos_sessao (sessao_id, hora_iso8601, tipo, descricao)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                self.sessao_id,
                hora_str,
                evento.tipo.label(),
                evento.descricao,
            ],
        )?;
        debug!(
            "Evento registrado: {} — {}",
            evento.tipo.label(),
            evento.descricao
        );
        Ok(())
    }

    /// Registra um bloco da cadeia de hashes.
    pub fn registrar_bloco_hash(&self, bloco: &ChainBlock) -> StoreResult<()> {
        self.db.conn.execute(
            "INSERT INTO cadeia_hashes (sessao_id, bloco_index, offset_bytes, block_hash, chain_hash)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                self.sessao_id,
                bloco.index as i64,
                bloco.offset_bytes as i64,
                bloco.block_hash,
                bloco.chain_hash,
            ],
        )?;
        Ok(())
    }

    /// Atualiza o estado da sessão.
    pub fn atualizar_estado(&self, estado: &str) -> StoreResult<()> {
        self.db.conn.execute(
            "UPDATE sessoes SET estado = ?1 WHERE id = ?2",
            params![estado, self.sessao_id],
        )?;
        Ok(())
    }

    /// Marca a sessão como encerrada regularmente.
    /// Deve ser chamado APÓS o flush final e o cálculo do hash.
    pub fn encerrar_regular(&self, fim_iso8601: &str, gravacao_id: &str) -> StoreResult<()> {
        self.db.conn.execute(
            "UPDATE sessoes SET encerramento_regular = 1, fim_iso8601 = ?1, gravacao_id = ?2, estado = 'concluida'
             WHERE id = ?3",
            params![fim_iso8601, gravacao_id, self.sessao_id],
        )?;
        info!("Sessão encerrada regularmente: {}", self.sessao_id);
        Ok(())
    }

    /// Marca a sessão como recuperada (sem encerramento regular).
    pub fn marcar_recuperada(&self) -> StoreResult<()> {
        self.db.conn.execute(
            "UPDATE sessoes SET encerramento_regular = 1, estado = 'recuperada' WHERE id = ?1",
            params![self.sessao_id],
        )?;
        Ok(())
    }
}

impl Database {
    /// Lista sessões sem encerramento regular (candidatas a recuperação).
    pub fn sessoes_pendentes(&self) -> StoreResult<Vec<SessaoPendente>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, depoente_nome, processo_numero, arquivo_path, inicio_iso8601
             FROM sessoes
             WHERE encerramento_regular = 0
             ORDER BY inicio_iso8601 DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(SessaoPendente {
                id: r.get(0)?,
                depoente_nome: r.get(1)?,
                processo_numero: r.get(2)?,
                arquivo_path: r.get(3)?,
                inicio_iso8601: r.get(4)?,
            })
        })?;
        let mut result = Vec::new();
        for row in rows {
            if let Ok(s) = row {
                result.push(s);
            }
        }
        Ok(result)
    }

    /// Carrega os eventos de uma sessão para geração do manifesto de recuperação.
    pub fn eventos_da_sessao(&self, sessao_id: &str) -> StoreResult<Vec<(String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT hora_iso8601, tipo, descricao FROM eventos_sessao
             WHERE sessao_id = ?1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![sessao_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        let mut result = Vec::new();
        for row in rows {
            if let Ok(e) = row {
                result.push(e);
            }
        }
        Ok(result)
    }

    /// Carrega os blocos da cadeia de hashes de uma sessão.
    pub fn cadeia_hashes_da_sessao(&self, sessao_id: &str) -> StoreResult<Vec<ChainBlock>> {
        let mut stmt = self.conn.prepare(
            "SELECT bloco_index, offset_bytes, block_hash, chain_hash
             FROM cadeia_hashes WHERE sessao_id = ?1 ORDER BY bloco_index ASC",
        )?;
        let rows = stmt.query_map(params![sessao_id], |r| {
            Ok(ChainBlock {
                index: r.get::<_, i64>(0)? as usize,
                offset_bytes: r.get::<_, i64>(1)? as u64,
                block_hash: r.get(2)?,
                chain_hash: r.get(3)?,
            })
        })?;
        let mut result = Vec::new();
        for row in rows {
            if let Ok(b) = row {
                result.push(b);
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clio_core::models::{EventoSessao, TipoEvento};
    use time::OffsetDateTime;

    #[test]
    fn diario_basico() {
        let db = Database::open_in_memory().unwrap();
        let journal = SessionJournal::iniciar(
            &db,
            "sess-001",
            "João Silva",
            "0001234",
            "/gravacoes/joao.mkv",
            "2026-11-17T14:00:00Z",
            None,
        )
        .unwrap();

        journal
            .registrar_evento(&EventoSessao {
                hora: OffsetDateTime::now_utc(),
                tipo: TipoEvento::InicioGravacao,
                descricao: "Gravação iniciada".to_string(),
            })
            .unwrap();

        let eventos = db.eventos_da_sessao("sess-001").unwrap();
        assert_eq!(eventos.len(), 1);
        assert!(eventos[0].2.contains("Gravação iniciada"));
    }

    #[test]
    fn sessoes_pendentes_detectadas() {
        let db = Database::open_in_memory().unwrap();

        // Criar sessão sem encerrar
        SessionJournal::iniciar(
            &db,
            "sess-crash",
            "Pedro",
            "0001",
            "/arq.mkv",
            "2026-11-17T10:00:00Z",
            None,
        )
        .unwrap();

        let pendentes = db.sessoes_pendentes().unwrap();
        assert_eq!(pendentes.len(), 1);
        assert_eq!(pendentes[0].id, "sess-crash");
    }

    #[test]
    fn encerramento_regular_remove_da_lista_pendentes() {
        let db = Database::open_in_memory().unwrap();

        let grav = crate::library::GravacaoInfo {
            id: "grav-001".to_string(),
            depoente_nome: "Ana".to_string(),
            depoente_qualificacao: "Testemunha".to_string(),
            processo_numero: "0002".to_string(),
            orgao: "MPTO".to_string(),
            inicio_iso8601: "2026-11-17T11:00:00Z".to_string(),
            fim_iso8601: Some("2026-11-17T12:00:00Z".to_string()),
            duracao_liquida_secs: Some(3600),
            arquivo_path: "/arq2.mkv".to_string(),
            arquivo_tamanho_bytes: Some(100),
            arquivo_sha256: Some("sha".to_string()),
            status: crate::library::StatusGravacao::Concluida,
            recuperada: false,
            manifesto_path: None,
        };
        db.registrar_gravacao(&grav).unwrap();

        let journal = SessionJournal::iniciar(
            &db,
            "sess-ok",
            "Ana",
            "0002",
            "/arq2.mkv",
            "2026-11-17T11:00:00Z",
            None,
        )
        .unwrap();

        journal
            .encerrar_regular("2026-11-17T12:00:00Z", "grav-001")
            .unwrap();

        let pendentes = db.sessoes_pendentes().unwrap();
        assert!(pendentes.is_empty());
    }
}
