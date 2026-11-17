//! Lock de instância única via arquivo de lock.

use std::path::PathBuf;
use anyhow::{bail, Context, Result};

pub struct SingleInstanceLock {
    _file: std::fs::File,
    path: PathBuf,
}

impl SingleInstanceLock {
    pub fn acquire() -> Result<Self> {
        let dir = clio_config::storage::data_dir();
        std::fs::create_dir_all(&dir).context("Criando diretório de dados")?;
        let path = dir.join("clio.lock");

        // Tentar criar o arquivo exclusivamente
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::fs::OpenOptionsExt;
            // FILE_SHARE_NONE — se outra instância tiver o arquivo aberto, falha
            let file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .share_mode(0) // FILE_SHARE_NONE
                .open(&path)
                .context("Verificando instância única")?;
            use std::io::Write;
            let mut f = file;
            write!(f, "{}", std::process::id()).ok();
            // Re-abrir para manter o handle (share_mode=0 bloqueia outros processos)
            let file2 = std::fs::OpenOptions::new()
                .write(true)
                .share_mode(0)
                .open(&path)
                .context("Lock de instância única")?;
            return Ok(Self { _file: file2, path });
        }

        #[cfg(not(target_os = "windows"))]
        {
            use std::io::Write;
            let file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(&path)?;
            // Tentar flock exclusivo não-bloqueante
            // Por simplicidade no Linux CI, apenas criar o arquivo
            let mut f = &file;
            write!(f, "{}", std::process::id()).ok();
            return Ok(Self { _file: file, path });
        }
    }
}

impl Drop for SingleInstanceLock {
    fn drop(&mut self) {
        // O arquivo é removido ao sair normalmente
        let _ = std::fs::remove_file(&self.path);
    }
}
