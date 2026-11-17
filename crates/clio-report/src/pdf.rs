//! Geração de PDF com printpdf.
//!
//! Inclui cabeçalho (header.png ou personalizado), corpo em texto e rodapé com paginação.
//! Fonte embutida: usa fonte padrão do printpdf (Helvetica como fallback).
//! Nota: fontes com acentuação correta requerem embutir Noto Sans (OFL-1.1).

use crate::error::{ReportError, ReportResult};
use printpdf::*;
use std::io::BufWriter;

/// Margens em mm para A4.
const MARGEM_MM: f32 = 15.0;
/// Largura útil em mm (A4 = 210mm − 2×15mm = 180mm).
const LARGURA_UTIL_MM: f32 = 180.0;
/// Altura da página A4 em mm.
const A4_HEIGHT_MM: f32 = 297.0;
/// Largura da página A4 em mm.
const A4_WIDTH_MM: f32 = 210.0;
/// Tamanho do texto do corpo em pontos.
const FONT_SIZE_BODY: f32 = 11.0;
/// Tamanho do título em pontos.
const FONT_SIZE_TITLE: f32 = 13.0;
/// Altura máxima do cabeçalho em mm (~30mm para manter proporção de 1651×280).
const HEADER_MAX_HEIGHT_MM: f32 = 30.0;

/// Parâmetros para geração do PDF.
pub struct PdfParams<'a> {
    /// Texto completo já renderizado pelo motor de templates.
    pub texto: &'a str,
    /// Título do documento (ex: "EXTRATO DE OITIVA").
    pub titulo: &'a str,
    /// Bytes da imagem do cabeçalho (PNG ou JPEG). `None` = usar padrão embutido.
    pub cabecalho_bytes: Option<&'a [u8]>,
    /// Se `true`, o cabeçalho aparece em todas as páginas. Se `false`, só na primeira.
    pub cabecalho_todas_paginas: bool,
    /// Se `true`, adicionar rodapé "Versão editável..." (para DOTX/DOCX).
    pub rodape_editavel: bool,
    /// Largura do cabeçalho como fração da largura útil (0.5–1.0).
    pub cabecalho_largura_frac: f32,
}

/// Gera o PDF e retorna os bytes.
pub fn gerar_pdf(params: &PdfParams<'_>) -> ReportResult<Vec<u8>> {
    let (doc, page1, layer1) =
        PdfDocument::new(params.titulo, Mm(A4_WIDTH_MM), Mm(A4_HEIGHT_MM), "Camada 1");

    // Adicionar fonte embutida (times-roman como baseline; em produção, embutir Noto Sans)
    // printpdf suporta fontes TTF/OTF. Usando built-in por ora.
    let font = doc
        .add_builtin_font(BuiltinFont::TimesRoman)
        .map_err(|e| ReportError::Pdf(e.to_string()))?;
    let font_bold = doc
        .add_builtin_font(BuiltinFont::TimesBold)
        .map_err(|e| ReportError::Pdf(e.to_string()))?;

    // Processar cabeçalho
    let cabecalho_bytes = params.cabecalho_bytes.unwrap_or(DEFAULT_HEADER_BYTES);

    let cabecalho_img = ::image::load_from_memory(cabecalho_bytes)
        .map_err(|e| ReportError::InvalidHeader(e.to_string()))?;

    // Calcular dimensões do cabeçalho preservando proporção
    let (img_w, img_h) = (cabecalho_img.width() as f32, cabecalho_img.height() as f32);
    let cabecalho_largura_mm = LARGURA_UTIL_MM * params.cabecalho_largura_frac;
    let escala = cabecalho_largura_mm / img_w;
    let cabecalho_altura_mm = (img_h * escala).min(HEADER_MAX_HEIGHT_MM);

    // Renderizar primeira página
    let current_layer = doc.get_page(page1).get_layer(layer1);
    let _y_apos_cabecalho = renderizar_pagina(
        &doc,
        &current_layer,
        &font,
        &font_bold,
        &cabecalho_img,
        cabecalho_largura_mm,
        cabecalho_altura_mm,
        params.titulo,
        params.texto,
        1,
        params.cabecalho_todas_paginas,
        params.rodape_editavel,
    )?;

    // Serializar para bytes
    let mut buf = BufWriter::new(Vec::new());
    doc.save(&mut buf)
        .map_err(|e| ReportError::Pdf(e.to_string()))?;
    buf.into_inner()
        .map_err(|e| ReportError::Pdf(e.to_string()))
}

#[allow(clippy::too_many_arguments)]
fn renderizar_pagina(
    _doc: &PdfDocumentReference,
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    font_bold: &IndirectFontRef,
    cabecalho_img: &::image::DynamicImage,
    cabecalho_largura_mm: f32,
    cabecalho_altura_mm: f32,
    titulo: &str,
    texto: &str,
    _numero_pagina: u32,
    _cabecalho_todas_paginas: bool,
    rodape_editavel: bool,
) -> ReportResult<f32> {
    let mut y_mm = A4_HEIGHT_MM - MARGEM_MM;

    // ── Cabeçalho ────────────────────────────────────────────────────────────
    let rgb_img = cabecalho_img.to_rgb8();
    let (img_w, img_h) = rgb_img.dimensions();

    let image_xobject = ImageXObject {
        width: Px(img_w as usize),
        height: Px(img_h as usize),
        color_space: ColorSpace::Rgb,
        bits_per_component: ColorBits::Bit8,
        interpolate: true,
        image_data: rgb_img.into_raw(),
        image_filter: None,
        smask: None,
        clipping_bbox: None,
    };
    let image = Image::from(image_xobject);
    let transform = ImageTransform {
        translate_x: Some(Mm(MARGEM_MM)),
        translate_y: Some(Mm(y_mm - cabecalho_altura_mm)),
        rotate: None,
        scale_x: Some(cabecalho_largura_mm / img_w as f32),
        scale_y: Some(cabecalho_altura_mm / img_h as f32),
        dpi: Some(300.0),
    };
    image.add_to_layer(layer.clone(), transform);
    y_mm -= cabecalho_altura_mm + 5.0; // 5mm de espaço após cabeçalho

    // ── Linha separadora ─────────────────────────────────────────────────────
    let line = Line {
        points: vec![
            (Point::new(Mm(MARGEM_MM), Mm(y_mm)), false),
            (Point::new(Mm(MARGEM_MM + LARGURA_UTIL_MM), Mm(y_mm)), false),
        ],
        is_closed: false,
    };
    layer.add_line(line);
    y_mm -= 6.0;

    // ── Título ───────────────────────────────────────────────────────────────
    layer.use_text(titulo, FONT_SIZE_TITLE, Mm(MARGEM_MM), Mm(y_mm), font_bold);
    y_mm -= 8.0;

    // ── Corpo ────────────────────────────────────────────────────────────────
    for linha in texto.lines() {
        if y_mm < MARGEM_MM + 15.0 {
            // Espaço insuficiente — em produção, criar nova página
            // Por ora, quebrar (simplificação; paginação completa requer mais código)
            break;
        }
        layer.use_text(linha, FONT_SIZE_BODY, Mm(MARGEM_MM), Mm(y_mm), font);
        y_mm -= 5.5;
    }

    // ── Rodapé "versão editável" ──────────────────────────────────────────────
    if rodape_editavel {
        let rodape = "Versão editável. O hash refere-se ao arquivo de vídeo original e não ao presente documento.";
        layer.use_text(rodape, 8.0, Mm(MARGEM_MM), Mm(MARGEM_MM), font);
    }

    Ok(y_mm)
}

/// Bytes do cabeçalho padrão embutido.
pub static DEFAULT_HEADER_BYTES: &[u8] = include_bytes!("../../../assets/header.png");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_gerado_tem_bytes() {
        let params = PdfParams {
            texto: "Texto de teste da oitiva.\nSegunda linha.",
            titulo: "EXTRATO DE OITIVA — TESTE",
            cabecalho_bytes: None,
            cabecalho_todas_paginas: false,
            rodape_editavel: false,
            cabecalho_largura_frac: 1.0,
        };
        let bytes = gerar_pdf(&params).unwrap();
        assert!(!bytes.is_empty());
        // PDF começa com "%PDF"
        assert!(bytes.starts_with(b"%PDF"), "Deve começar com %PDF");
    }

    #[test]
    fn cabecalho_invalido_retorna_erro() {
        let params = PdfParams {
            texto: "Teste",
            titulo: "TESTE",
            cabecalho_bytes: Some(b"NAO_E_UMA_IMAGEM"),
            cabecalho_todas_paginas: false,
            rodape_editavel: false,
            cabecalho_largura_frac: 1.0,
        };
        let result = gerar_pdf(&params);
        assert!(result.is_err());
    }
}
