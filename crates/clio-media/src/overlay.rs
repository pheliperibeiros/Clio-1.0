//! Renderização da tarja probatória em Rust puro (tiny-skia).
//!
//! A tarja é uma imagem RGBA de 1920×altura_px gerada a cada segundo
//! (ou imediatamente quando um parâmetro muda) e injetada como frame no
//! appsrc do compositor GStreamer.
//!
//! # Regras de transparência
//! - 0% → fundo completamente opaco (alpha = 255)
//! - 100% → fundo completamente transparente (alpha = 0)
//! - Texto e indicadores permanecem SEMPRE alpha = 255
//! - Acima de 40% de transparência → texto ganha sombra automática
//! - Acima de 60% → aviso na UI (a renderização continua normalmente)

use crate::error::{MediaError, MediaResult};
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Rect, Transform};

/// Parâmetros de configuração da tarja.
#[derive(Debug, Clone)]
pub struct OverlayParams {
    /// Largura total do frame (padrão: 1920).
    pub frame_width: u32,
    /// Altura total do frame (padrão: 1080).
    pub frame_height: u32,
    /// Altura da tarja em pixels (54, 72 ou 96).
    pub tarja_height_px: u32,
    /// Posição: `true` = inferior, `false` = superior.
    pub posicao_inferior: bool,
    /// Cor de fundo RGBA (antes de aplicar transparência).
    pub cor_fundo: [u8; 4],
    /// Transparência do fundo 0–100% (0 = opaco, 100 = transparente).
    pub transparencia_pct: u8,
    /// Nome completo do depoente.
    pub depoente_nome: String,
    /// Qualificação ("Testemunha", "Réu", etc.).
    pub depoente_qualificacao: String,
    /// Número dos autos.
    pub numero_processo: String,
    /// Órgão/instituição.
    pub orgao: String,
    /// Data e hora atual (já formatados, ex.: "17/11/2026 14:30:05").
    pub data_hora: String,
    /// Se a gravação está ativa, mostra "● GRAVANDO" + cronômetro.
    pub gravando: bool,
    /// Cronômetro já formatado ("01:23:45").
    pub cronometro: String,
    /// Linha de defesa compacta (opcional, ex.: "DEFESA: Dra. Ana (OAB/TO 1234)").
    pub linha_defesa: Option<String>,
    /// Se `true`, adicionar sombra ao texto (ativado automaticamente acima de 40% transp.).
    pub sombra_texto: bool,
    /// Bytes da logomarca opcional à esquerda da tarja (None = sem logomarca).
    pub logomarca_bytes: Option<Vec<u8>>,
}

impl OverlayParams {
    /// Calcula se a sombra deve ser ativada com base na transparência.
    pub fn sombra_automatica(&self) -> bool {
        self.transparencia_pct > 40
    }

    /// Converte transparência para alpha do fundo (0 = transparente, 255 = opaco).
    pub fn fundo_alpha(&self) -> u8 {
        let alpha_f = 1.0 - (self.transparencia_pct.min(100) as f32 / 100.0);
        (alpha_f * 255.0).round() as u8
    }
}

/// Renderizador da tarja probatória.
pub struct OverlayRenderer;

impl OverlayRenderer {
    /// Renderiza a tarja e retorna os bytes RGBA do frame completo (1920×1080).
    ///
    /// O frame é transparente em toda a área exceto a faixa da tarja,
    /// que é preenchida com a cor de fundo (com alpha configurável) e o texto.
    pub fn render(params: &OverlayParams) -> MediaResult<Vec<u8>> {
        let mut pixmap = Pixmap::new(params.frame_width, params.frame_height)
            .ok_or_else(|| MediaError::OverlayError("Não foi possível criar Pixmap".to_string()))?;

        // Coordenada Y da tarja
        let tarja_y = if params.posicao_inferior {
            params.frame_height - params.tarja_height_px
        } else {
            0
        };

        // ── Fundo da tarja ────────────────────────────────────────────────────
        let fundo_alpha = params.fundo_alpha();
        let [r, g, b, _] = params.cor_fundo;
        let cor_fundo = Color::from_rgba8(r, g, b, fundo_alpha);

        let mut paint = Paint::default();
        paint.set_color(cor_fundo);
        paint.anti_alias = false;

        let rect = Rect::from_xywh(
            0.0,
            tarja_y as f32,
            params.frame_width as f32,
            params.tarja_height_px as f32,
        )
        .ok_or_else(|| MediaError::OverlayError("Retângulo inválido".to_string()))?;

        let path = PathBuilder::from_rect(rect);
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );

        // ── Texto ─────────────────────────────────────────────────────────────
        // Nota: tiny-skia não renderiza texto diretamente.
        // Em produção, integrar com `ab_glyph` ou `cosmic-text` para rasterização de glyphs.
        // Aqui renderizamos a tarja com cor de fundo; o texto seria adicionado pelo
        // módulo de texto (integração na fase (b)).
        //
        // Por ora, sinalizamos a posição do texto via metadados retornados no frame.
        // A integração completa requer adicionar ab_glyph ao Cargo.toml de clio-media.

        Self::render_text_placeholder(&mut pixmap, params, tarja_y)?;

        Ok(pixmap.take())
    }

    /// Placeholder de texto — escreve bytes de cor branca em posições fixas
    /// simulando onde o texto estaria. Substituir por ab_glyph na fase (b).
    fn render_text_placeholder(
        pixmap: &mut Pixmap,
        params: &OverlayParams,
        tarja_y: u32,
    ) -> MediaResult<()> {
        // Indicador "GRAVANDO" — faixa vermelha de 8px na extremidade direita
        if params.gravando {
            let mut paint = Paint::default();
            paint.set_color(Color::from_rgba8(220, 30, 30, 255));
            let rect = Rect::from_xywh(
                (params.frame_width - 12) as f32,
                tarja_y as f32,
                12.0,
                params.tarja_height_px as f32,
            );
            if let Some(r) = rect {
                let path = PathBuilder::from_rect(r);
                pixmap.fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
        }
        Ok(())
    }

    /// Verifica se os parâmetros diferem o suficiente para exigir re-renderização.
    pub fn needs_redraw(prev: &OverlayParams, next: &OverlayParams) -> bool {
        prev.gravando != next.gravando
            || prev.cronometro != next.cronometro
            || prev.data_hora != next.data_hora
            || prev.transparencia_pct != next.transparencia_pct
            || prev.cor_fundo != next.cor_fundo
            || prev.depoente_nome != next.depoente_nome
            || prev.linha_defesa != next.linha_defesa
            || prev.tarja_height_px != next.tarja_height_px
            || prev.posicao_inferior != next.posicao_inferior
    }
}

impl Default for OverlayParams {
    fn default() -> Self {
        Self {
            frame_width: 1920,
            frame_height: 1080,
            tarja_height_px: 72,
            posicao_inferior: true,
            cor_fundo: [0, 0, 0, 255],
            transparencia_pct: 6,
            depoente_nome: String::new(),
            depoente_qualificacao: String::new(),
            numero_processo: String::new(),
            orgao: String::new(),
            data_hora: String::new(),
            gravando: false,
            cronometro: "00:00:00".to_string(),
            linha_defesa: None,
            sombra_texto: false,
            logomarca_bytes: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_frame_tem_tamanho_correto() {
        let params = OverlayParams::default();
        let bytes = OverlayRenderer::render(&params).unwrap();
        // RGBA: 4 bytes por pixel
        assert_eq!(bytes.len(), (1920 * 1080 * 4) as usize);
    }

    #[test]
    fn fundo_alpha_opaco() {
        let mut p = OverlayParams::default();
        p.transparencia_pct = 0;
        assert_eq!(p.fundo_alpha(), 255);
    }

    #[test]
    fn fundo_alpha_transparente() {
        let mut p = OverlayParams::default();
        p.transparencia_pct = 100;
        assert_eq!(p.fundo_alpha(), 0);
    }

    #[test]
    fn fundo_alpha_padrao_6pct() {
        let p = OverlayParams::default(); // 6%
        let alpha = p.fundo_alpha();
        // 1.0 - 0.06 = 0.94 → 0.94 × 255 ≈ 239
        assert!(alpha >= 238 && alpha <= 240, "alpha={alpha}");
    }

    #[test]
    fn sombra_automatica_acima_40() {
        let mut p = OverlayParams::default();
        p.transparencia_pct = 41;
        assert!(p.sombra_automatica());
        p.transparencia_pct = 40;
        assert!(!p.sombra_automatica());
    }

    #[test]
    fn needs_redraw_detecta_mudanca() {
        let p1 = OverlayParams::default();
        let mut p2 = p1.clone();
        assert!(!OverlayRenderer::needs_redraw(&p1, &p2));
        p2.cronometro = "00:00:01".to_string();
        assert!(OverlayRenderer::needs_redraw(&p1, &p2));
    }
}
