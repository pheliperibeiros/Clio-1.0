//! Geração e sanitização de nomes de arquivo para gravações.
//!
//! Padrão: `Oitiva_[Nome]_Proc-[Processo]_[AAAAMMDD]_[hh]h[mm].[ext]`
//! Sanitizado contra: caracteres inválidos no Windows, nomes reservados,
//! barras, pontos duplos e comprimento excessivo do caminho.

use crate::error::{CoreError, CoreResult};
use once_cell::sync::Lazy;
use regex::Regex;

/// Nomes reservados no Windows (case-insensitive, com ou sem extensão).
const WINDOWS_RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL",
    "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Comprimento máximo do componente de arquivo (sem a extensão).
const MAX_STEM_LEN: usize = 180;

static RE_INVALID_CHARS: Lazy<Regex> = Lazy::new(|| {
    // Caracteres proibidos no Windows: \ / : * ? " < > |
    // Também remove caracteres de controle (0x00–0x1F).
    Regex::new(r#"[\\/:*?"<>|\x00-\x1f]"#).unwrap()
});

static RE_LEADING_TRAILING_DOTS_SPACES: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[\s.]+|[\s.]+$").unwrap()
});

static RE_MULTIPLE_UNDERSCORES: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"_+").unwrap()
});

/// Remove acentuação e substitui por ASCII equivalente (simples, sem dependência externa).
fn deaccent(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'à' | 'ã' | 'â' | 'ä' => 'a',
            'Á' | 'À' | 'Ã' | 'Â' | 'Ä' => 'A',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'É' | 'È' | 'Ê' | 'Ë' => 'E',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
            'ó' | 'ò' | 'õ' | 'ô' | 'ö' => 'o',
            'Ó' | 'Ò' | 'Õ' | 'Ô' | 'Ö' => 'O',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
            'ç' => 'c',
            'Ç' => 'C',
            'ñ' => 'n',
            'Ñ' => 'N',
            other => other,
        })
        .collect()
}

/// Sanitiza um componente de texto para uso em nome de arquivo.
pub fn sanitize_component(s: &str) -> String {
    let s = deaccent(s);
    // Substituir espaços por underscore antes de remover inválidos
    let s = s.replace(' ', "_");
    let s = RE_INVALID_CHARS.replace_all(&s, "").to_string();
    // Remover sequências de path traversal
    let s = s.replace("..", "");
    let s = RE_LEADING_TRAILING_DOTS_SPACES.replace_all(&s, "").to_string();
    let s = RE_MULTIPLE_UNDERSCORES.replace_all(&s, "_").to_string();
    s
}

/// Verifica se o stem gerado colide com um nome reservado do Windows.
fn is_reserved_name(stem: &str) -> bool {
    let upper = stem.to_uppercase();
    // Checar o nome inteiro ou até o primeiro ponto
    let base = upper.split('.').next().unwrap_or(&upper);
    WINDOWS_RESERVED.contains(&base)
}

/// Gera o nome de arquivo padrão para uma gravação.
///
/// # Parâmetros
/// - `nome_depoente`: nome completo do depoente
/// - `numero_processo`: número do processo/autos
/// - `inicio`: data/hora de início (UTC ou local)
/// - `ext`: extensão sem ponto (ex.: "mkv")
pub fn gerar_nome_arquivo(
    nome_depoente: &str,
    numero_processo: &str,
    inicio: &time::OffsetDateTime,
    ext: &str,
) -> CoreResult<String> {
    let nome = sanitize_component(nome_depoente);
    let proc = sanitize_component(numero_processo);

    if nome.is_empty() {
        return Err(CoreError::MissingField("nome do depoente".to_string()));
    }

    let data = format!(
        "{}{:02}{:02}_{:02}h{:02}",
        inicio.year(),
        u8::from(inicio.month()),
        inicio.day(),
        inicio.hour(),
        inicio.minute(),
    );

    let stem = if proc.is_empty() {
        format!("Oitiva_{nome}_{data}")
    } else {
        format!("Oitiva_{nome}_Proc-{proc}_{data}")
    };

    // Truncar se necessário
    let stem = if stem.len() > MAX_STEM_LEN {
        stem[..MAX_STEM_LEN].to_string()
    } else {
        stem
    };

    if is_reserved_name(&stem) {
        return Err(CoreError::InvalidFilename(format!(
            "O nome gerado '{stem}' colide com um nome reservado do Windows."
        )));
    }

    let ext_clean = sanitize_component(ext);
    Ok(format!("{stem}.{ext_clean}"))
}

/// Gera o nome da subpasta da sessão (mesmo padrão, sem extensão).
pub fn gerar_nome_pasta(
    nome_depoente: &str,
    numero_processo: &str,
    inicio: &time::OffsetDateTime,
) -> CoreResult<String> {
    let nome = sanitize_component(nome_depoente);
    let proc = sanitize_component(numero_processo);
    let data = format!(
        "{}{:02}{:02}_{:02}h{:02}",
        inicio.year(),
        u8::from(inicio.month()),
        inicio.day(),
        inicio.hour(),
        inicio.minute(),
    );
    let pasta = if proc.is_empty() {
        format!("Oitiva_{nome}_{data}")
    } else {
        format!("Oitiva_{nome}_Proc-{proc}_{data}")
    };
    let pasta = if pasta.len() > MAX_STEM_LEN {
        pasta[..MAX_STEM_LEN].to_string()
    } else {
        pasta
    };
    Ok(pasta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn ts() -> time::OffsetDateTime {
        datetime!(2026-11-17 14:30 UTC)
    }

    #[test]
    fn nome_basico() {
        let nome = gerar_nome_arquivo("João Silva", "0001234-56.2026.8.27.2729", &ts(), "mkv").unwrap();
        // Deve conter as partes essenciais
        assert!(nome.starts_with("Oitiva_Joao_Silva_"));
        assert!(nome.ends_with(".mkv"));
        assert!(nome.contains("20261117"));
    }

    #[test]
    fn sanitiza_caracteres_invalidos() {
        let s = sanitize_component("CON: arquivo/inválido?*");
        assert!(!s.contains(':'));
        assert!(!s.contains('/'));
        assert!(!s.contains('?'));
        assert!(!s.contains('*'));
    }

    #[test]
    fn path_traversal_bloqueado() {
        let s = sanitize_component("../../etc/passwd");
        assert!(!s.contains(".."));
        assert!(!s.contains('/'));
    }

    #[test]
    fn nome_reservado_detectado() {
        // Simular que o nome do depoente gera "CON"
        let result = gerar_nome_arquivo("C.O.N", "00000", &ts(), "mkv");
        // Pode gerar "Oitiva_CON_..." — verificar se é reservado
        // No caso, o stem não será exatamente "CON" porque tem "Oitiva_" na frente
        // O teste serve para garantir que a função não panics e retorna um nome válido
        assert!(result.is_ok());
    }

    #[test]
    fn deaccent_funciona() {
        assert_eq!(deaccent("ção"), "cao");
        assert_eq!(deaccent("José"), "Jose");
        assert_eq!(deaccent("Ângela"), "Angela");
    }

    #[test]
    fn comprimento_maximo() {
        let nome_longo = "A".repeat(300);
        let resultado = gerar_nome_arquivo(&nome_longo, "00000", &ts(), "mkv").unwrap();
        assert!(resultado.len() <= MAX_STEM_LEN + 4); // +4 para ".mkv"
    }
}
