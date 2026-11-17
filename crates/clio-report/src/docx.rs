//! Geração de DOCX e DOTX com OOXML manual (zip + quick-xml).
//!
//! O cabeçalho (header.png) é embutido em `word/header1.xml` como imagem de cabeçalho Word,
//! não como imagem solta no corpo — garantindo edição limpa e estilo correto.
//!
//! Validação: o ZIP gerado deve abrir no Word e no LibreOffice sem aviso de reparo.

use crate::error::{ReportError, ReportResult};
use std::io::{Cursor, Write};
use zip::{ZipWriter, write::FileOptions};

/// Tipo de documento OOXML.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocxTipo {
    /// `.docx` — documento Word editável.
    Docx,
    /// `.dotx` — modelo Word (ao abrir, cria novo documento sem sobrescrever o modelo).
    Dotx,
}

/// Parâmetros para geração de DOCX/DOTX.
pub struct DocxParams<'a> {
    pub tipo: DocxTipo,
    /// Texto completo já renderizado.
    pub texto: &'a str,
    /// Título do documento.
    pub titulo: &'a str,
    /// Bytes da imagem do cabeçalho (PNG). `None` = usar padrão embutido.
    pub cabecalho_bytes: Option<&'a [u8]>,
    /// Se `true`, o cabeçalho aparece em todas as páginas.
    pub cabecalho_todas_paginas: bool,
    /// Se `true`, adicionar rodapé "Versão editável...".
    pub rodape_editavel: bool,
    /// Largura do cabeçalho como % da largura útil (50–100).
    pub cabecalho_largura_pct: u8,
}

/// Gera DOCX ou DOTX e retorna os bytes.
pub fn gerar_docx(params: &DocxParams<'_>) -> ReportResult<Vec<u8>> {
    let cabecalho_bytes = params
        .cabecalho_bytes
        .unwrap_or(crate::pdf::DEFAULT_HEADER_BYTES);

    // Validar imagem
    image::load_from_memory(cabecalho_bytes)
        .map_err(|e| ReportError::InvalidHeader(e.to_string()))?;

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts: FileOptions<()> =
        FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // ── [Content_Types].xml ───────────────────────────────────────────────────
    zip.start_file("[Content_Types].xml", opts)
        .map_err(|e| ReportError::Docx(e.to_string()))?;
    write!(zip, "{}", content_types_xml(params.tipo))
        .map_err(|e| ReportError::Docx(e.to_string()))?;

    // ── _rels/.rels ───────────────────────────────────────────────────────────
    zip.start_file("_rels/.rels", opts)
        .map_err(|e| ReportError::Docx(e.to_string()))?;
    write!(zip, "{}", rels_xml()).map_err(|e| ReportError::Docx(e.to_string()))?;

    // ── word/_rels/document.xml.rels ─────────────────────────────────────────
    zip.start_file("word/_rels/document.xml.rels", opts)
        .map_err(|e| ReportError::Docx(e.to_string()))?;
    write!(zip, "{}", document_rels_xml(params.cabecalho_todas_paginas))
        .map_err(|e| ReportError::Docx(e.to_string()))?;

    // ── word/media/header.png ─────────────────────────────────────────────────
    zip.start_file("word/media/header.png", opts)
        .map_err(|e| ReportError::Docx(e.to_string()))?;
    zip.write_all(cabecalho_bytes)
        .map_err(|e| ReportError::Docx(e.to_string()))?;

    // ── word/header1.xml ─────────────────────────────────────────────────────
    zip.start_file("word/header1.xml", opts)
        .map_err(|e| ReportError::Docx(e.to_string()))?;
    write!(zip, "{}", header1_xml(params.cabecalho_largura_pct))
        .map_err(|e| ReportError::Docx(e.to_string()))?;

    // ── word/_rels/header1.xml.rels ───────────────────────────────────────────
    zip.start_file("word/_rels/header1.xml.rels", opts)
        .map_err(|e| ReportError::Docx(e.to_string()))?;
    write!(zip, "{}", header1_rels_xml()).map_err(|e| ReportError::Docx(e.to_string()))?;

    // ── word/document.xml ─────────────────────────────────────────────────────
    zip.start_file("word/document.xml", opts)
        .map_err(|e| ReportError::Docx(e.to_string()))?;
    write!(
        zip,
        "{}",
        document_xml(
            params.titulo,
            params.texto,
            params.cabecalho_todas_paginas,
            params.rodape_editavel,
        )
    )
    .map_err(|e| ReportError::Docx(e.to_string()))?;

    // ── word/styles.xml ───────────────────────────────────────────────────────
    zip.start_file("word/styles.xml", opts)
        .map_err(|e| ReportError::Docx(e.to_string()))?;
    write!(zip, "{}", styles_xml()).map_err(|e| ReportError::Docx(e.to_string()))?;

    // ── word/settings.xml ─────────────────────────────────────────────────────
    zip.start_file("word/settings.xml", opts)
        .map_err(|e| ReportError::Docx(e.to_string()))?;
    write!(zip, "{}", settings_xml()).map_err(|e| ReportError::Docx(e.to_string()))?;

    // ── docProps/core.xml ─────────────────────────────────────────────────────
    zip.start_file("docProps/core.xml", opts)
        .map_err(|e| ReportError::Docx(e.to_string()))?;
    write!(zip, "{}", core_props_xml(params.titulo))
        .map_err(|e| ReportError::Docx(e.to_string()))?;

    zip.finish()
        .map(|c| c.into_inner())
        .map_err(|e| ReportError::Docx(e.to_string()))
}

// ─── Fragmentos OOXML ────────────────────────────────────────────────────────

fn content_types_xml(tipo: DocxTipo) -> String {
    let doc_content_type = match tipo {
        DocxTipo::Docx => {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"
        }
        DocxTipo::Dotx => {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.template.main+xml"
        }
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="png" ContentType="image/png"/>
  <Override PartName="/word/document.xml" ContentType="{doc_content_type}"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
  <Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/>
  <Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/>
  <Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
</Types>"#
    )
}

fn rels_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
</Relationships>"#
}

fn document_rels_xml(_todas_paginas: bool) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/>
</Relationships>"#
    )
}

fn header1_rels_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/header.png"/>
</Relationships>"#
}

/// Cabeçalho Word com a imagem header.png.
/// `largura_pct`: percentual da largura útil (100% = ~8.84 cm em A4 com margens 15mm → 18 cm).
/// A largura em EMUs: 1 mm = 36000 EMUs; 180 mm × (pct/100) × 36000.
fn header1_xml(largura_pct: u8) -> String {
    let pct = largura_pct.clamp(50, 100) as u32;
    // A4 útil = 180mm; em EMUs (1 cm = 360000 EMU, 1 mm = 36000 EMU)
    let largura_emu: u64 = 180 * 36000 * pct as u64 / 100;
    // Proporção header.png: 1651×280 → altura ≈ largura × 280/1651
    let altura_emu: u64 = largura_emu * 280 / 1651;

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
       xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:p>
    <w:pPr><w:pStyle w:val="Header"/></w:pPr>
    <w:r>
      <w:rPr/>
      <w:drawing>
        <wp:inline distT="0" distB="0" distL="0" distR="0">
          <wp:extent cx="{largura_emu}" cy="{altura_emu}"/>
          <wp:effectExtent l="0" t="0" r="0" b="0"/>
          <wp:docPr id="1" name="Cabecalho" descr="Cabeçalho institucional"/>
          <wp:cNvGraphicFramePr>
            <a:graphicFrameLocks xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" noChangeAspect="1"/>
          </wp:cNvGraphicFramePr>
          <a:graphic>
            <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
              <pic:pic>
                <pic:nvPicPr>
                  <pic:cNvPr id="1" name="header.png"/>
                  <pic:cNvPicPr/>
                </pic:nvPicPr>
                <pic:blipFill>
                  <a:blip r:embed="rId1"/>
                  <a:stretch><a:fillRect/></a:stretch>
                </pic:blipFill>
                <pic:spPr>
                  <a:xfrm><a:off x="0" y="0"/><a:ext cx="{largura_emu}" cy="{altura_emu}"/></a:xfrm>
                  <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
                </pic:spPr>
              </pic:pic>
            </a:graphicData>
          </a:graphic>
        </wp:inline>
      </w:drawing>
    </w:r>
  </w:p>
</w:hdr>"#
    )
}

fn document_xml(titulo: &str, texto: &str, cabecalho_todas: bool, rodape_editavel: bool) -> String {
    let header_ref = if cabecalho_todas {
        r#"<w:headerReference w:type="default" r:id="rId3"/>"#
    } else {
        r#"<w:headerReference w:type="first" r:id="rId3"/><w:titlePg/>"#
    };

    // Escapar XML no título e texto
    let titulo_esc = escape_xml(titulo);

    // Converter linhas do texto em parágrafos Word
    let paragrafos: String = texto
        .lines()
        .map(|l| {
            if l.trim().is_empty() {
                "<w:p><w:pPr><w:pStyle w:val=\"Normal\"/></w:pPr></w:p>".to_string()
            } else {
                format!(
                    "<w:p><w:pPr><w:pStyle w:val=\"Normal\"/></w:pPr><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
                    escape_xml(l)
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let rodape_xml = if rodape_editavel {
        "<w:p><w:pPr><w:pStyle w:val=\"Footer\"/></w:pPr>\
         <w:r><w:t>Versão editável. O hash refere-se ao arquivo de vídeo original e não ao presente documento.</w:t></w:r></w:p>"
    } else {
        ""
    };

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body>
    <w:sectPr>
      {header_ref}
      <w:pgSz w:w="11906" w:h="16838"/>
      <w:pgMar w:top="851" w:right="851" w:bottom="851" w:left="851"
               w:header="709" w:footer="709" w:gutter="0"/>
    </w:sectPr>
    <w:p><w:pPr><w:pStyle w:val="Titulo"/></w:pPr>
      <w:r><w:t>{titulo_esc}</w:t></w:r>
    </w:p>
    {paragrafos}
    {rodape_xml}
  </w:body>
</w:document>"#
    )
}

fn styles_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
          xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml">
  <w:docDefaults>
    <w:rPrDefault>
      <w:rPr>
        <w:rFonts w:ascii="Times New Roman" w:hAnsi="Times New Roman"/>
        <w:sz w:val="22"/>
        <w:lang w:val="pt-BR"/>
      </w:rPr>
    </w:rPrDefault>
  </w:docDefaults>
  <w:style w:type="paragraph" w:styleId="Normal" w:default="1">
    <w:name w:val="Normal"/>
    <w:pPr><w:spacing w:after="120"/></w:pPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Titulo">
    <w:name w:val="Titulo"/>
    <w:basedOn w:val="Normal"/>
    <w:pPr><w:jc w:val="center"/><w:spacing w:before="240" w:after="240"/></w:pPr>
    <w:rPr><w:b/><w:sz w:val="28"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Assinatura">
    <w:name w:val="Assinatura"/>
    <w:basedOn w:val="Normal"/>
    <w:pPr><w:jc w:val="center"/><w:spacing w:before="480" w:after="120"/></w:pPr>
    <w:rPr><w:b/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Header">
    <w:name w:val="Header"/>
    <w:basedOn w:val="Normal"/>
    <w:pPr><w:jc w:val="center"/></w:pPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Footer">
    <w:name w:val="Footer"/>
    <w:basedOn w:val="Normal"/>
    <w:pPr><w:jc w:val="center"/></w:pPr>
    <w:rPr><w:i/><w:sz w:val="18"/></w:rPr>
  </w:style>
</w:styles>"#
}

fn settings_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:defaultTabStop w:val="720"/>
  <w:themeFontLang w:val="pt-BR"/>
</w:settings>"#
}

fn core_props_xml(titulo: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/">
  <dc:title>{}</dc:title>
  <dc:creator>Clio — Gravador de Oitivas e Audiências</dc:creator>
  <dc:language>pt-BR</dc:language>
</cp:coreProperties>"#,
        escape_xml(titulo)
    )
}

/// Escapa caracteres especiais XML.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::ZipArchive;

    fn gerar_docx_teste(tipo: DocxTipo) -> Vec<u8> {
        let params = DocxParams {
            tipo,
            texto: "Linha 1 do documento.\nLinha 2 do documento.",
            titulo: "EXTRATO DE OITIVA — TESTE",
            cabecalho_bytes: None,
            cabecalho_todas_paginas: true,
            rodape_editavel: tipo == DocxTipo::Dotx,
            cabecalho_largura_pct: 100,
        };
        gerar_docx(&params).unwrap()
    }

    #[test]
    fn docx_e_zip_valido() {
        let bytes = gerar_docx_teste(DocxTipo::Docx);
        let cursor = Cursor::new(bytes);
        let archive = ZipArchive::new(cursor).expect("Deve ser ZIP válido");
        assert!(archive.len() > 0);
    }

    #[test]
    fn docx_contem_arquivos_obrigatorios() {
        let bytes = gerar_docx_teste(DocxTipo::Docx);
        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor).unwrap();
        let nomes: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(nomes.contains(&"[Content_Types].xml".to_string()));
        assert!(nomes.contains(&"word/document.xml".to_string()));
        assert!(nomes.contains(&"word/header1.xml".to_string()));
        assert!(nomes.contains(&"word/media/header.png".to_string()));
    }

    #[test]
    fn dotx_tem_content_type_correto() {
        let bytes = gerar_docx_teste(DocxTipo::Dotx);
        let cursor = Cursor::new(bytes.clone());
        let mut archive = ZipArchive::new(cursor).unwrap();
        let mut ct = archive.by_name("[Content_Types].xml").unwrap();
        let mut content = String::new();
        std::io::Read::read_to_string(&mut ct, &mut content).unwrap();
        assert!(
            content.contains("template.main+xml"),
            "DOTX deve ter content type de template: {content}"
        );
    }

    #[test]
    fn docx_tem_content_type_correto() {
        let bytes = gerar_docx_teste(DocxTipo::Docx);
        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor).unwrap();
        let mut ct = archive.by_name("[Content_Types].xml").unwrap();
        let mut content = String::new();
        std::io::Read::read_to_string(&mut ct, &mut content).unwrap();
        assert!(
            content.contains("document.main+xml"),
            "DOCX deve ter content type de documento: {content}"
        );
    }

    #[test]
    fn header1_xml_contem_imagem() {
        let bytes = gerar_docx_teste(DocxTipo::Docx);
        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor).unwrap();
        let mut h = archive.by_name("word/header1.xml").unwrap();
        let mut content = String::new();
        std::io::Read::read_to_string(&mut h, &mut content).unwrap();
        assert!(
            content.contains("pic:pic"),
            "header1.xml deve conter imagem"
        );
        assert!(
            content.contains("rId1"),
            "header1.xml deve referenciar rId1"
        );
    }

    #[test]
    fn escape_xml_correto() {
        assert_eq!(escape_xml("a & b < c > d"), "a &amp; b &lt; c &gt; d");
        assert_eq!(escape_xml("\"aspas\""), "&quot;aspas&quot;");
    }
}
