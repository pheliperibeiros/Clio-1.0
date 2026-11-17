//! Medidor VU (Volume Unit) por canal de microfone.
//!
//! Recebe amostras RMS do GStreamer (via `level` element) e expõe o nível atual,
//! o pico de curta duração e o alerta de clipping.

use std::time::{Duration, Instant};

/// Nível VU de um microfone.
#[derive(Debug, Clone, Copy)]
pub struct VuLevel {
    /// Nível RMS atual em dBFS (geralmente −60 a 0).
    pub rms_dbfs: f32,
    /// Pico de curta duração em dBFS (decai lentamente).
    pub peak_dbfs: f32,
    /// `true` se o sinal atingiu clipping (pico ≥ −0.1 dBFS).
    pub clipping: bool,
    /// `true` se o sinal ficou abaixo do limiar por mais de 10 s (possível mute no hardware).
    pub silence_alert: bool,
}

impl Default for VuLevel {
    fn default() -> Self {
        Self {
            rms_dbfs: -60.0,
            peak_dbfs: -60.0,
            clipping: false,
            silence_alert: false,
        }
    }
}

impl VuLevel {
    /// Nível normalizado 0.0–1.0 para barra VU na UI.
    pub fn rms_normalized(&self) -> f32 {
        // Mapear −60 dBFS..0 dBFS → 0.0..1.0
        ((self.rms_dbfs + 60.0) / 60.0).clamp(0.0, 1.0)
    }

    pub fn peak_normalized(&self) -> f32 {
        ((self.peak_dbfs + 60.0) / 60.0).clamp(0.0, 1.0)
    }
}

/// Acumulador de nível VU para um canal.
pub struct VuAccumulator {
    pub level: VuLevel,
    /// Tempo do último update acima do limiar de silêncio.
    last_signal: Instant,
    /// Limiar de silêncio em dBFS (padrão: −50 dBFS).
    silence_threshold_dbfs: f32,
    /// Duração antes de emitir alerta de silêncio.
    silence_duration: Duration,
    /// Decaimento do pico por update (dB/s × dt).
    peak_decay_per_sec: f32,
    last_update: Instant,
}

impl VuAccumulator {
    pub fn new(silence_threshold_dbfs: f32, silence_secs: u64) -> Self {
        Self {
            level: VuLevel::default(),
            last_signal: Instant::now(),
            silence_threshold_dbfs,
            silence_duration: Duration::from_secs(silence_secs),
            peak_decay_per_sec: 20.0, // 20 dB/s de decaimento do pico
            last_update: Instant::now(),
        }
    }

    /// Atualiza com novos valores do elemento `level` do GStreamer.
    pub fn update(&mut self, rms_dbfs: f32, peak_dbfs: f32) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f32();
        self.last_update = now;

        self.level.rms_dbfs = rms_dbfs;

        // Pico: só sobe, decai lentamente
        if peak_dbfs > self.level.peak_dbfs {
            self.level.peak_dbfs = peak_dbfs;
        } else {
            self.level.peak_dbfs -= self.peak_decay_per_sec * dt;
            self.level.peak_dbfs = self.level.peak_dbfs.max(-60.0);
        }

        self.level.clipping = peak_dbfs >= -0.1;

        if rms_dbfs > self.silence_threshold_dbfs {
            self.last_signal = now;
        }

        self.level.silence_alert =
            now.duration_since(self.last_signal) >= self.silence_duration;
    }

    /// Reseta o acumulador (ex.: ao mutar o microfone).
    pub fn reset(&mut self) {
        self.level = VuLevel::default();
        self.last_signal = Instant::now();
    }
}

impl Default for VuAccumulator {
    fn default() -> Self {
        Self::new(-50.0, 10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_normalizado() {
        let mut v = VuLevel::default();
        v.rms_dbfs = -60.0;
        assert!((v.rms_normalized() - 0.0).abs() < 0.01);
        v.rms_dbfs = 0.0;
        assert!((v.rms_normalized() - 1.0).abs() < 0.01);
        v.rms_dbfs = -30.0;
        assert!((v.rms_normalized() - 0.5).abs() < 0.01);
    }

    #[test]
    fn clipping_detectado() {
        let mut acc = VuAccumulator::default();
        acc.update(-1.0, -0.05); // pico quase 0 dBFS
        assert!(acc.level.clipping);
    }

    #[test]
    fn silencio_nao_imediato() {
        let mut acc = VuAccumulator::new(-50.0, 10);
        acc.update(-55.0, -55.0); // abaixo do limiar
        // Mas ainda não passou 10 s
        assert!(!acc.level.silence_alert);
    }
}
