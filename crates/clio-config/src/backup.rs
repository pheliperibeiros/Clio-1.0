//! Backup rotativo dos últimos N arquivos de configuração.

use crate::error::{ConfigError, ConfigResult};
use std::path::{Path, PathBuf};

pub const MAX_BACKUPS: usize = 5;

/// Rotaciona os backups: renomeia `config.toml.bak.N` → `config.toml.bak.N+1`,
/// depois copia o arquivo atual para `config.toml.bak.1`.
pub fn rotacionar_backups(config_path: &Path) -> ConfigResult<()> {
    // Remover o backup mais antigo se já houver MAX_BACKUPS
    let oldest = backup_path(config_path, MAX_BACKUPS);
    if oldest.exists() {
        std::fs::remove_file(&oldest)?;
    }
    // Deslocar backups existentes
    for i in (1..MAX_BACKUPS).rev() {
        let src = backup_path(config_path, i);
        let dst = backup_path(config_path, i + 1);
        if src.exists() {
            std::fs::rename(&src, &dst)?;
        }
    }
    // Copiar atual para bak.1
    if config_path.exists() {
        std::fs::copy(config_path, backup_path(config_path, 1))?;
    }
    Ok(())
}

/// Restaura o backup mais recente válido.
/// Retorna o caminho do backup restaurado, ou erro se nenhum for válido.
pub fn restaurar_backup_valido(config_path: &Path) -> ConfigResult<PathBuf> {
    for i in 1..=MAX_BACKUPS {
        let bak = backup_path(config_path, i);
        if bak.exists() {
            // Tentar parse para verificar validade
            let content = std::fs::read_to_string(&bak)?;
            if toml::from_str::<toml::Value>(&content).is_ok() {
                // Backup válido encontrado — copiá-lo sobre o original
                std::fs::copy(&bak, config_path)?;
                return Ok(bak);
            }
        }
    }
    Err(ConfigError::NoValidBackup)
}

pub fn backup_path(config_path: &Path, n: usize) -> PathBuf {
    let mut p = config_path.to_path_buf();
    let nome = p
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    p.set_file_name(format!("{nome}.bak.{n}"));
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rotacionar_cria_bak1() {
        let dir = std::env::temp_dir().join("clio_backup_test");
        std::fs::create_dir_all(&dir).unwrap();
        let config = dir.join("config.toml");
        let mut f = std::fs::File::create(&config).unwrap();
        writeln!(f, "schema_version = 3").unwrap();
        drop(f);

        rotacionar_backups(&config).unwrap();

        let bak1 = backup_path(&config, 1);
        assert!(bak1.exists(), "bak.1 deve existir após rotação");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
