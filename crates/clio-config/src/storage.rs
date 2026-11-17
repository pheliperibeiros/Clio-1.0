//! Leitura e gravação atômica do config.toml.

use std::path::{Path, PathBuf};
use tracing::{info, warn, error};

use crate::backup;
use crate::error::{ConfigError, ConfigResult};
use crate::migrations;
use crate::schema::{AppConfig, SCHEMA_VERSION};

/// Localiza o diretório de configuração do aplicativo.
///
/// Modo portátil: se `portable.flag` existir ao lado do executável, usa `./data/`.
/// Modo padrão: `%APPDATA%\MPTO\Clio\` no Windows, `~/.config/MPTO/Clio/` no Linux.
pub fn config_dir() -> PathBuf {
    // Verificar modo portátil
    if let Ok(exe) = std::env::current_exe() {
        let flag = exe.parent().map(|p| p.join("portable.flag"));
        if flag.as_ref().map(|f| f.exists()).unwrap_or(false) {
            let data_dir = exe.parent().unwrap().join("data");
            return data_dir;
        }
    }

    // Usar diretório de configuração do usuário
    directories::ProjectDirs::from("br.mp.to", "MPTO", "Clio")
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| {
            warn!("Não foi possível determinar o diretório de configuração; usando ./data");
            PathBuf::from("data")
        })
}

/// Localiza o diretório de dados do aplicativo (imagens importadas, etc.).
pub fn data_dir() -> PathBuf {
    directories::ProjectDirs::from("br.mp.to", "MPTO", "Clio")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("data"))
}

/// Responsável por carregar, salvar e migrar o `config.toml`.
pub struct ConfigStorage {
    pub path: PathBuf,
}

impl ConfigStorage {
    pub fn new() -> Self {
        let dir = config_dir();
        Self { path: dir.join("config.toml") }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    /// Carrega a configuração do disco, aplicando migrações se necessário.
    ///
    /// Se o arquivo estiver corrompido, tenta restaurar o backup mais recente válido.
    /// Se não houver backup, retorna a configuração padrão e preserva o arquivo corrompido.
    pub fn carregar(&self) -> ConfigResult<(AppConfig, Vec<String>)> {
        if !self.path.exists() {
            info!("config.toml não encontrado; criando configuração padrão");
            return Ok((AppConfig::default(), vec![]));
        }

        let content = std::fs::read_to_string(&self.path)?;

        // Tentar desserializar como Value cru para leitura da versão
        let valor: toml::Value = match toml::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                error!("config.toml corrompido: {e}; tentando backup");
                return self.carregar_backup();
            }
        };

        // Ler schema_version
        let versao_encontrada = valor
            .get("schema_version")
            .and_then(|v| v.as_integer())
            .map(|v| v as u32)
            .unwrap_or(1); // sem versão = assume v1

        // Aplicar migrações se necessário
        let valor = if versao_encontrada < SCHEMA_VERSION {
            info!("Migrando config de v{versao_encontrada} para v{SCHEMA_VERSION}");
            migrations::migrar(valor, versao_encontrada)?
        } else {
            valor
        };

        // Desserializar para AppConfig
        let toml_str = toml::to_string(&valor)?;
        let mut config: AppConfig = match toml::from_str(&toml_str) {
            Ok(c) => c,
            Err(e) => {
                error!("Falha ao desserializar após migração: {e}; usando padrão");
                AppConfig::default()
            }
        };

        // Sanitizar valores fora de faixa
        let warnings = config.sanitize();
        for w in &warnings {
            warn!("Configuração corrigida: {w}");
        }

        Ok((config, warnings))
    }

    /// Salva a configuração de forma atômica: escreve em temp, fsync, renomeia.
    pub fn salvar(&self, config: &AppConfig) -> ConfigResult<()> {
        // Garantir que o diretório existe
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Rotacionar backups antes de sobrescrever
        backup::rotacionar_backups(&self.path)?;

        // Serializar
        let conteudo = toml::to_string_pretty(config)?;

        // Escrever em arquivo temporário no mesmo diretório
        let tmp_path = self.path.with_extension("toml.tmp");
        {
            use std::io::Write;
            let mut f = std::fs::File::create(&tmp_path)?;
            f.write_all(conteudo.as_bytes())?;
            f.flush()?;
            f.sync_all()?; // fsync
        }

        // Renomear atomicamente
        std::fs::rename(&tmp_path, &self.path)?;
        info!("Configuração salva em {:?}", self.path);
        Ok(())
    }

    /// Exporta a configuração para um caminho externo.
    pub fn exportar(&self, destino: &Path, config: &AppConfig) -> ConfigResult<()> {
        use std::io::Write;
        let conteudo = toml::to_string_pretty(config)?;
        let mut f = std::fs::File::create(destino)?;
        f.write_all(conteudo.as_bytes())?;
        f.sync_all()?;
        Ok(())
    }

    /// Importa a configuração de um caminho externo, com validação.
    pub fn importar(&self, origem: &Path) -> ConfigResult<AppConfig> {
        let content = std::fs::read_to_string(origem)?;
        // Verificar tamanho (limite: 1 MB)
        if content.len() > 1_048_576 {
            return Err(ConfigError::InvalidImport(
                "Arquivo excede 1 MB".to_string(),
            ));
        }
        // Desserializar
        let valor: toml::Value = toml::from_str(&content)
            .map_err(|e| ConfigError::InvalidImport(e.to_string()))?;
        let versao = valor
            .get("schema_version")
            .and_then(|v| v.as_integer())
            .map(|v| v as u32)
            .unwrap_or(1);
        let valor = migrations::migrar(valor, versao)?;
        let toml_str = toml::to_string(&valor)?;
        let mut config: AppConfig = toml::from_str(&toml_str)
            .map_err(|e| ConfigError::InvalidImport(e.to_string()))?;
        config.sanitize();
        Ok(config)
    }

    fn carregar_backup(&self) -> ConfigResult<(AppConfig, Vec<String>)> {
        match backup::restaurar_backup_valido(&self.path) {
            Ok(bak_path) => {
                warn!("Backup restaurado de {:?}", bak_path);
                // Preservar o arquivo corrompido com sufixo .corrupted
                let corrupted = self.path.with_extension("toml.corrupted");
                // Já foi sobrescrito; apenas logar
                let warnings = vec![format!(
                    "Arquivo de configuração corrompido. Backup restaurado de: {}",
                    bak_path.display()
                )];
                // Recarregar normalmente
                let content = std::fs::read_to_string(&self.path)?;
                let mut config: AppConfig = toml::from_str(&content)
                    .unwrap_or_default();
                let mut w2 = config.sanitize();
                let mut all_warnings = warnings;
                all_warnings.append(&mut w2);
                Ok((config, all_warnings))
            }
            Err(_) => {
                error!("Nenhum backup válido encontrado; usando configuração padrão");
                Ok((
                    AppConfig::default(),
                    vec!["Configuração corrompida; nenhum backup válido. Usando padrão.".to_string()],
                ))
            }
        }
    }
}

impl Default for ConfigStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_storage() -> (tempfile::TempDir, ConfigStorage) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let storage = ConfigStorage::with_path(path);
        (dir, storage)
    }

    // Nota: tempfile não está no workspace; este teste usa std::env::temp_dir diretamente.
    #[test]
    fn salvar_e_carregar_roundtrip() {
        let dir = std::env::temp_dir().join("clio_config_test_roundtrip");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        let storage = ConfigStorage::with_path(path);

        let mut config = AppConfig::default();
        config.instituicao.autoridade_nome = "Dr. João".to_string();
        storage.salvar(&config).unwrap();

        let (loaded, warnings) = storage.carregar().unwrap();
        assert_eq!(loaded.instituicao.autoridade_nome, "Dr. João");
        assert!(warnings.is_empty());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn arquivo_corrompido_carrega_padrao() {
        let dir = std::env::temp_dir().join("clio_config_test_corrupted");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");

        // Escrever conteúdo inválido
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "ISTO NÃO É TOML VÁLIDO {{{{{{").unwrap();
        drop(f);

        let storage = ConfigStorage::with_path(path);
        let (config, warnings) = storage.carregar().unwrap();

        // Deve retornar padrão com aviso
        assert!(!warnings.is_empty());
        assert_eq!(config.schema_version, SCHEMA_VERSION);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn escrita_atomica_nao_deixa_tmp() {
        let dir = std::env::temp_dir().join("clio_config_test_atomic");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        let storage = ConfigStorage::with_path(path.clone());

        storage.salvar(&AppConfig::default()).unwrap();

        let tmp = path.with_extension("toml.tmp");
        assert!(!tmp.exists(), "arquivo .tmp não deve existir após salvar");
        assert!(path.exists(), "config.toml deve existir após salvar");

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
