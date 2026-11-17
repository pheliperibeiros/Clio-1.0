//! Validações de domínio: OAB, número de processo, campos obrigatórios.

use crate::error::{CoreError, CoreResult};
use once_cell::sync::Lazy;
use regex::Regex;

/// Regex para OAB no formato "OAB/UF NNNN" ou "OAB/UF NNNNN" (com ou sem o prefixo "OAB/").
/// Aceita: "OAB/TO 1234", "TO 1234", "1234/TO".
/// O validador AVISA se inválido mas NÃO BLOQUEIA (conforme especificação).
static RE_OAB: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?xi)
        ^(?:OAB/)?                              # prefixo opcional
        ([A-Z]{2})\s+(\d{3,6})                 # UF + número (3 a 6 dígitos)
        $
        |
        ^(\d{3,6})\s*/\s*([A-Z]{2})            # número / UF (invertido)
        $
        ",
    )
    .unwrap()
});

/// Siglas de estados brasileiros válidos.
const ESTADOS_VALIDOS: &[&str] = &[
    "AC", "AL", "AP", "AM", "BA", "CE", "DF", "ES", "GO", "MA", "MT", "MS", "MG",
    "PA", "PB", "PR", "PE", "PI", "RJ", "RN", "RS", "RO", "RR", "SC", "SP", "SE", "TO",
];

/// Valida o formato de um número de OAB.
///
/// Retorna `Ok(())` se válido.
/// Retorna `Err(CoreError::InvalidOab)` se inválido — mas a UI deve apenas **avisar**, não bloquear.
pub fn validar_oab(oab: &str) -> CoreResult<()> {
    let oab = oab.trim();
    if oab.is_empty() {
        return Ok(()); // campo opcional
    }
    if !RE_OAB.is_match(oab) {
        return Err(CoreError::InvalidOab(oab.to_string()));
    }
    // Verificar se a UF é válida
    // Extrair a UF do match
    let uf = extract_uf(oab);
    if let Some(uf) = uf {
        if !ESTADOS_VALIDOS.contains(&uf.to_uppercase().as_str()) {
            return Err(CoreError::InvalidOab(format!(
                "{oab} — estado '{uf}' não reconhecido"
            )));
        }
    }
    Ok(())
}

fn extract_uf(oab: &str) -> Option<&str> {
    // Tentar "OAB/TO 1234" ou "TO 1234"
    let oab = oab.trim();
    let oab = oab.strip_prefix("OAB/").unwrap_or(oab);
    let oab = oab.trim();
    // Pegar as primeiras 2 letras se for formato UF primeiro
    if oab.len() >= 2 {
        let candidate = &oab[..2];
        if candidate.chars().all(|c| c.is_ascii_alphabetic()) {
            return Some(candidate);
        }
    }
    None
}

/// Valida que um campo de texto obrigatório não está vazio.
pub fn validar_obrigatorio(valor: &str, campo: &str) -> CoreResult<()> {
    if valor.trim().is_empty() {
        Err(CoreError::MissingField(campo.to_string()))
    } else {
        Ok(())
    }
}

/// Valida que a transparência está no intervalo [0, 100].
pub fn validar_transparencia(pct: u8) -> CoreResult<()> {
    if pct > 100 {
        return Err(CoreError::OutOfRange {
            field: "transparencia_fundo_pct".to_string(),
            value: pct.to_string(),
            min: "0".to_string(),
            max: "100".to_string(),
        });
    }
    Ok(())
}

/// Converte transparência (0–100%) em alpha (0.0–1.0) para renderização.
/// 0% = totalmente opaco (alpha = 1.0); 100% = totalmente transparente (alpha = 0.0).
pub fn transparencia_para_alpha(pct: u8) -> f32 {
    1.0 - (pct.min(100) as f32 / 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oab_valida() {
        assert!(validar_oab("OAB/TO 1234").is_ok());
        assert!(validar_oab("TO 1234").is_ok());
        assert!(validar_oab("").is_ok()); // vazio é ok (opcional)
    }

    #[test]
    fn oab_invalida() {
        assert!(validar_oab("XPTO").is_err());
        assert!(validar_oab("OAB 1234").is_err()); // sem UF
        assert!(validar_oab("OAB/XX 1234").is_err()); // UF inválida
    }

    #[test]
    fn transparencia_alpha_conversion() {
        assert!((transparencia_para_alpha(0) - 1.0).abs() < f32::EPSILON);
        assert!((transparencia_para_alpha(100) - 0.0).abs() < f32::EPSILON);
        assert!((transparencia_para_alpha(6) - 0.94).abs() < 0.01); // padrão = 6%
        assert!((transparencia_para_alpha(50) - 0.5).abs() < 0.01);
    }

    #[test]
    fn transparencia_fora_de_faixa() {
        assert!(validar_transparencia(101).is_err());
        assert!(validar_transparencia(100).is_ok());
    }
}
