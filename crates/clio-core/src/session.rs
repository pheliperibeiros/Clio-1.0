//! Tipos relativos à sessão de gravação em andamento.

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Intervalo de pausa numa sessão.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntervaloRecesso {
    /// Instante em que a pausa foi iniciada (epoch Unix em segundos).
    pub inicio_unix: i64,
    /// Instante em que a pausa foi encerrada (`None` se ainda ativa).
    pub fim_unix: Option<i64>,
}

impl IntervaloRecesso {
    /// Duração deste intervalo em segundos (0 se ainda ativo).
    pub fn duracao_secs(&self) -> u64 {
        match self.fim_unix {
            Some(fim) => (fim - self.inicio_unix).max(0) as u64,
            None => 0,
        }
    }
}

/// Acumulador de duração de pausas, operando com `std::time::Instant` em runtime.
#[derive(Debug)]
pub struct ContadorRecessos {
    intervalos: Vec<IntervaloRecesso>,
    inicio_pausa_atual: Option<Instant>,
    total_secs: u64,
}

impl ContadorRecessos {
    pub fn new() -> Self {
        Self {
            intervalos: Vec::new(),
            inicio_pausa_atual: None,
            total_secs: 0,
        }
    }

    pub fn iniciar_pausa(&mut self, agora_unix: i64) {
        if self.inicio_pausa_atual.is_none() {
            self.inicio_pausa_atual = Some(Instant::now());
            self.intervalos.push(IntervaloRecesso {
                inicio_unix: agora_unix,
                fim_unix: None,
            });
        }
    }

    pub fn encerrar_pausa(&mut self, agora_unix: i64) {
        if let Some(inicio) = self.inicio_pausa_atual.take() {
            let duracao = inicio.elapsed().as_secs();
            self.total_secs += duracao;
            if let Some(last) = self.intervalos.last_mut() {
                last.fim_unix = Some(agora_unix);
            }
        }
    }

    pub fn total_pausado_secs(&self) -> u64 {
        self.total_secs
            + self
                .inicio_pausa_atual
                .map(|i| i.elapsed().as_secs())
                .unwrap_or(0)
    }

    pub fn num_pausas(&self) -> u32 {
        self.intervalos.len() as u32
    }

    pub fn intervalos(&self) -> &[IntervaloRecesso] {
        &self.intervalos
    }
}

impl Default for ContadorRecessos {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contador_recessos_basico() {
        let mut c = ContadorRecessos::new();
        assert_eq!(c.num_pausas(), 0);
        c.iniciar_pausa(1000);
        assert_eq!(c.num_pausas(), 1);
        // Sem encerrar ainda, total_pausado_secs retorna algo >= 0
        assert!(c.total_pausado_secs() < 5); // deve ser quase zero
        c.encerrar_pausa(1030);
        assert!(c.total_pausado_secs() < 5); // ainda < 5s real, pois foi imediato
        assert_eq!(c.num_pausas(), 1);
    }

    #[test]
    fn intervalo_duracao() {
        let i = IntervaloRecesso {
            inicio_unix: 100,
            fim_unix: Some(130),
        };
        assert_eq!(i.duracao_secs(), 30);
    }

    #[test]
    fn intervalo_ativo_sem_fim() {
        let i = IntervaloRecesso {
            inicio_unix: 100,
            fim_unix: None,
        };
        assert_eq!(i.duracao_secs(), 0);
    }
}
