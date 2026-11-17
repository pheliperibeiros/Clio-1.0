//! Abertura e migração do banco de dados SQLite principal.

use rusqlite::{Connection, params};
use tracing::info;
use crate::error::StoreResult;

pub const DB_VERSION: u32 = 1;

/// Wrapper sobre a conexão SQLite com as configurações de durabilidade.
pub struct Database {
    pub conn: Connection,
}

impl Database {
    /// Abre (ou cria) o banco de dados no caminho especificado.
    /// Aplica WAL e synchronous=FULL.
    pub fn open(path: &std::path::Path) -> StoreResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        // Configurações de durabilidade
        conn.execute_batch("
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = FULL;
            PRAGMA foreign_keys = ON;
            PRAGMA temp_store = MEMORY;
        ")?;
        let mut db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Abre o banco em memória (apenas para testes).
    pub fn open_in_memory() -> StoreResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = FULL;
            PRAGMA foreign_keys = ON;
        ")?;
        let mut db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&mut self) -> StoreResult<()> {
        // Criar tabela de versão se não existir
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER NOT NULL
            );
        ")?;
        let version: u32 = self.conn
            .query_row(
                "SELECT version FROM schema_version LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        if version < DB_VERSION {
            self.apply_migrations(version)?;
        }
        Ok(())
    }

    fn apply_migrations(&mut self, from_version: u32) -> StoreResult<()> {
        let mut v = from_version;
        while v < DB_VERSION {
            match v {
                0 => self.migrate_v0_to_v1()?,
                _ => break,
            }
            v += 1;
        }
        // Atualizar versão
        self.conn.execute("DELETE FROM schema_version", [])?;
        self.conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            params![DB_VERSION],
        )?;
        info!("Banco de dados migrado para versão {DB_VERSION}");
        Ok(())
    }

    fn migrate_v0_to_v1(&mut self) -> StoreResult<()> {
        self.conn.execute_batch("
            -- Biblioteca de gravações concluídas
            CREATE TABLE IF NOT EXISTS gravacoes (
                id TEXT PRIMARY KEY,
                depoente_nome TEXT NOT NULL,
                depoente_qualificacao TEXT NOT NULL,
                processo_numero TEXT NOT NULL,
                orgao TEXT NOT NULL,
                inicio_iso8601 TEXT NOT NULL,
                fim_iso8601 TEXT,
                duracao_liquida_secs INTEGER,
                arquivo_path TEXT NOT NULL,
                arquivo_tamanho_bytes INTEGER,
                arquivo_sha256 TEXT,
                status TEXT NOT NULL DEFAULT 'concluida',
                recuperada INTEGER NOT NULL DEFAULT 0,
                manifesto_path TEXT,
                metadados_json TEXT,
                criada_em TEXT NOT NULL DEFAULT (datetime('now'))
            );

            -- Diário de sessões (criado antes de iniciar o pipeline)
            CREATE TABLE IF NOT EXISTS sessoes (
                id TEXT PRIMARY KEY,
                gravacao_id TEXT REFERENCES gravacoes(id),
                depoente_nome TEXT NOT NULL,
                processo_numero TEXT NOT NULL,
                arquivo_path TEXT NOT NULL,
                estado TEXT NOT NULL DEFAULT 'iniciando',
                inicio_iso8601 TEXT NOT NULL,
                fim_iso8601 TEXT,
                metadados_json TEXT,
                encerramento_regular INTEGER NOT NULL DEFAULT 0,
                criada_em TEXT NOT NULL DEFAULT (datetime('now'))
            );

            -- Eventos da sessão
            CREATE TABLE IF NOT EXISTS eventos_sessao (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sessao_id TEXT NOT NULL REFERENCES sessoes(id),
                hora_iso8601 TEXT NOT NULL,
                tipo TEXT NOT NULL,
                descricao TEXT NOT NULL
            );

            -- Cadeia de hashes por sessão
            CREATE TABLE IF NOT EXISTS cadeia_hashes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sessao_id TEXT NOT NULL REFERENCES sessoes(id),
                bloco_index INTEGER NOT NULL,
                offset_bytes INTEGER NOT NULL,
                block_hash TEXT NOT NULL,
                chain_hash TEXT NOT NULL
            );

            -- Exportações (documentos gerados)
            CREATE TABLE IF NOT EXISTS exportacoes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                gravacao_id TEXT NOT NULL REFERENCES gravacoes(id),
                formato TEXT NOT NULL,
                tipo TEXT NOT NULL,  -- 'extrato' ou 'relatorio'
                arquivo_path TEXT NOT NULL,
                arquivo_sha256 TEXT,
                gerado_em TEXT NOT NULL DEFAULT (datetime('now'))
            );

            -- Índices úteis
            CREATE INDEX IF NOT EXISTS idx_gravacoes_depoente ON gravacoes(depoente_nome);
            CREATE INDEX IF NOT EXISTS idx_gravacoes_processo ON gravacoes(processo_numero);
            CREATE INDEX IF NOT EXISTS idx_gravacoes_inicio ON gravacoes(inicio_iso8601);
            CREATE INDEX IF NOT EXISTS idx_sessoes_encerramento ON sessoes(encerramento_regular);
            CREATE INDEX IF NOT EXISTS idx_eventos_sessao ON eventos_sessao(sessao_id);
        ")?;
        Ok(())
    }
}
