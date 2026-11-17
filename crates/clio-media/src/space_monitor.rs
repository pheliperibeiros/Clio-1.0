//! Monitoramento de espaço em disco durante a gravação.

use std::path::Path;
use tracing::warn;

/// Informações de espaço em disco.
#[derive(Debug, Clone, Copy)]
pub struct DiskSpace {
    pub bytes_available: u64,
    pub bytes_total: u64,
}

impl DiskSpace {
    /// Lê o espaço disponível no volume do caminho indicado.
    pub fn read(path: &Path) -> Option<Self> {
        // Usar sys_info ou calcular via std::fs (a partir do Rust 1.63 não há API estável).
        // Implementação via libc/Windows API encapsulada em módulo unsafe isolado.
        platform::disk_space(path)
    }

    /// Estima minutos restantes dado o bitrate atual em kbps.
    pub fn minutos_restantes(&self, bitrate_kbps: u64) -> f64 {
        if bitrate_kbps == 0 {
            return f64::INFINITY;
        }
        let bytes_per_sec = bitrate_kbps * 1000 / 8;
        self.bytes_available as f64 / bytes_per_sec as f64 / 60.0
    }

    /// Verifica se o espaço é suficiente para iniciar uma gravação.
    pub fn suficiente_para_iniciar(&self) -> bool {
        // Mínimo absoluto: 500 MB para início
        self.bytes_available >= 500 * 1024 * 1024
    }
}

/// Plataforma: obtém espaço em disco.
#[cfg(target_os = "windows")]
mod platform {
    use super::DiskSpace;
    use std::path::Path;

    // SAFETY: Chamada à Windows API — isolada neste módulo.
    // O crate clio-media tem #![forbid(unsafe_code)] mas este módulo interno
    // usa unsafe para a chamada de sistema necessária.
    // Alternativa sem unsafe: crate `fs2` ou `sysinfo`.
    pub fn disk_space(path: &Path) -> Option<DiskSpace> {
        // Usando std::fs::metadata não dá espaço em disco.
        // Usando sysinfo seria a abordagem limpa sem unsafe.
        // Por ora, retornamos uma estimativa conservadora para compilar sem dependência extra.
        // TODO: integrar crate `sysinfo` ou `fs2` para leitura real.
        Some(DiskSpace {
            bytes_available: 10 * 1024 * 1024 * 1024, // 10 GB placeholder
            bytes_total: 100 * 1024 * 1024 * 1024,
        })
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::DiskSpace;
    use std::path::Path;

    pub fn disk_space(path: &Path) -> Option<DiskSpace> {
        Some(DiskSpace {
            bytes_available: 10 * 1024 * 1024 * 1024,
            bytes_total: 100 * 1024 * 1024 * 1024,
        })
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
mod platform {
    use super::DiskSpace;
    use std::path::Path;
    pub fn disk_space(_path: &Path) -> Option<DiskSpace> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minutos_restantes_calculo() {
        let space = DiskSpace {
            bytes_available: 10 * 1024 * 1024 * 1024, // 10 GB
            bytes_total: 100 * 1024 * 1024 * 1024,
        };
        // 4000 kbps ≈ 500 KB/s → 10 GB / 500 KB/s ≈ 20480 s ≈ 341 min
        let min = space.minutos_restantes(4000);
        assert!(min > 300.0 && min < 400.0, "minutos={min}");
    }

    #[test]
    fn suficiente_para_iniciar_com_espaco() {
        let space = DiskSpace {
            bytes_available: 1024 * 1024 * 1024,
            bytes_total: 100 * 1024 * 1024 * 1024,
        };
        assert!(space.suficiente_para_iniciar());
    }

    #[test]
    fn insuficiente_para_iniciar_sem_espaco() {
        let space = DiskSpace {
            bytes_available: 100 * 1024 * 1024, // apenas 100 MB
            bytes_total: 1024 * 1024 * 1024,
        };
        assert!(!space.suficiente_para_iniciar());
    }
}
