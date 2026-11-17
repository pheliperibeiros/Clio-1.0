//! Wake lock — impede a suspensão do computador durante a gravação.
//!
//! No Windows: `SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED)`.
//! Em outros sistemas: no-op (documentado).
//!
//! # Nota sobre unsafe
//! Este é o único módulo do workspace que usa `unsafe`, isolado aqui e comentado.
//! A chamada é necessária porque não há crate seguro equivalente disponível no
//! registro oficial sem dependências de rede ou licenças problemáticas.
//! Alternativa futura: crate `keepawake` (verificar licença antes de adicionar).

use tracing::{info, warn};

/// Guarda do wake lock — libera automaticamente ao ser dropado.
pub struct WakeLock {
    _private: (),
}

impl WakeLock {
    /// Ativa o wake lock. Retorna `None` se não suportado na plataforma.
    pub fn acquire() -> Option<Self> {
        platform::acquire_wake_lock()
    }
}

impl Drop for WakeLock {
    fn drop(&mut self) {
        platform::release_wake_lock();
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;

    // Constantes da Windows API
    const ES_CONTINUOUS: u32 = 0x8000_0000;
    const ES_SYSTEM_REQUIRED: u32 = 0x0000_0001;
    const ES_DISPLAY_REQUIRED: u32 = 0x0000_0002;

    // SAFETY: `SetThreadExecutionState` é uma chamada à Windows API documentada,
    // sem risco de corrupção de memória — recebe apenas um valor inteiro e retorna
    // o estado anterior. É chamada a partir de uma única thread de controle.
    #[link(name = "kernel32")]
    extern "system" {
        fn SetThreadExecutionState(esFlags: u32) -> u32;
    }

    pub fn acquire_wake_lock() -> Option<WakeLock> {
        // SAFETY: ver comentário acima.
        let prev = unsafe {
            SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED)
        };
        if prev == 0 {
            warn!("SetThreadExecutionState falhou — suspensão não impedida");
            None
        } else {
            info!("Wake lock ativado — suspensão impedida durante a gravação");
            Some(WakeLock { _private: () })
        }
    }

    pub fn release_wake_lock() {
        // SAFETY: restaura o estado anterior; veja nota acima.
        unsafe {
            SetThreadExecutionState(ES_CONTINUOUS);
        }
        info!("Wake lock liberado");
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use super::*;

    pub fn acquire_wake_lock() -> Option<WakeLock> {
        warn!("Wake lock não implementado nesta plataforma — suspensão não impedida");
        Some(WakeLock { _private: () })
    }

    pub fn release_wake_lock() {}
}
