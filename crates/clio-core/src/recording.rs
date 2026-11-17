//! Máquina de estados da gravação.
//!
//! Transições válidas:
//!   Idle → PreCheck → Ready → Recording → Paused → Recording (loop)
//!   Recording → Finalizing → Done
//!   Paused → Finalizing → Done
//!   Qualquer → Error

use serde::{Deserialize, Serialize};
use crate::error::{CoreError, CoreResult};

/// Estado atual da sessão de gravação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecordingState {
    /// Nenhuma sessão ativa. Aguardando configuração.
    Idle,
    /// Verificações pré-gravação em andamento (câmeras, espaço, etc.).
    PreCheck,
    /// Verificações passaram; pronto para iniciar.
    Ready,
    /// Gravando ativamente.
    Recording,
    /// Gravação pausada (recesso). Pipeline continua; encoder descarta frames.
    Paused,
    /// Finalizando (flush, hash, manifesto).
    Finalizing,
    /// Sessão encerrada com sucesso.
    Done,
    /// Erro irrecuperável.
    Error,
}

impl RecordingState {
    /// Retorna `true` se a transição para `next` é válida.
    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Idle, Self::PreCheck)
                | (Self::PreCheck, Self::Ready)
                | (Self::PreCheck, Self::Idle)   // cancelar
                | (Self::Ready, Self::Recording)
                | (Self::Ready, Self::Idle)       // cancelar
                | (Self::Recording, Self::Paused)
                | (Self::Recording, Self::Finalizing)
                | (Self::Paused, Self::Recording)
                | (Self::Paused, Self::Finalizing)
                | (Self::Finalizing, Self::Done)
                | (Self::Finalizing, Self::Error)
                | (_, Self::Error)
        )
    }

    /// Tenta fazer a transição, retornando erro descritivo se inválida.
    pub fn transition(self, next: Self) -> CoreResult<Self> {
        if self.can_transition_to(next) {
            Ok(next)
        } else {
            Err(CoreError::InvalidStateTransition { from: self, to: next })
        }
    }

    /// Indica se a gravação está ativa (escrevendo no disco).
    pub fn is_active(self) -> bool {
        matches!(self, Self::Recording | Self::Paused)
    }

    /// Label em pt-BR para exibição na UI.
    pub fn label_ptbr(self) -> &'static str {
        match self {
            Self::Idle => "Aguardando",
            Self::PreCheck => "Verificando",
            Self::Ready => "Pronto",
            Self::Recording => "Gravando",
            Self::Paused => "Pausado (Recesso)",
            Self::Finalizing => "Finalizando",
            Self::Done => "Concluído",
            Self::Error => "Erro",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_transitions() {
        assert!(RecordingState::Idle.can_transition_to(RecordingState::PreCheck));
        assert!(RecordingState::PreCheck.can_transition_to(RecordingState::Ready));
        assert!(RecordingState::Ready.can_transition_to(RecordingState::Recording));
        assert!(RecordingState::Recording.can_transition_to(RecordingState::Paused));
        assert!(RecordingState::Paused.can_transition_to(RecordingState::Recording));
        assert!(RecordingState::Recording.can_transition_to(RecordingState::Finalizing));
        assert!(RecordingState::Finalizing.can_transition_to(RecordingState::Done));
    }

    #[test]
    fn invalid_transitions() {
        assert!(!RecordingState::Idle.can_transition_to(RecordingState::Recording));
        assert!(!RecordingState::Done.can_transition_to(RecordingState::Recording));
        assert!(!RecordingState::Finalizing.can_transition_to(RecordingState::Recording));
    }

    #[test]
    fn error_transition_always_allowed() {
        for state in [
            RecordingState::Idle,
            RecordingState::Recording,
            RecordingState::Done,
        ] {
            assert!(state.can_transition_to(RecordingState::Error));
        }
    }

    #[test]
    fn transition_method_returns_error_on_invalid() {
        let result = RecordingState::Done.transition(RecordingState::Recording);
        assert!(result.is_err());
    }
}
