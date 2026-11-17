//! Filtros customizados para o motor de templates.
//!
//! Registrados no `minijinja::Environment` pelo `TemplateEngine`.

use minijinja::{Value, Error as JinjaError, ErrorKind};

/// `data_longa`: "17 de novembro de 2026"
pub fn filtro_data_longa(val: Value, _args: &[Value]) -> Result<Value, JinjaError> {
    let s = val.as_str().unwrap_or("");
    match parse_iso_date(s) {
        Some((year, month, day)) => {
            let mes = nome_mes(month);
            Ok(Value::from(format!("{day} de {mes} de {year}")))
        }
        None => Ok(val), // Retorna o valor original sem erro
    }
}

/// `data`: "17/11/2026"
pub fn filtro_data(val: Value, _args: &[Value]) -> Result<Value, JinjaError> {
    let s = val.as_str().unwrap_or("");
    match parse_iso_date(s) {
        Some((year, month, day)) => Ok(Value::from(format!("{day:02}/{month:02}/{year}"))),
        None => Ok(val),
    }
}

/// `hora`: "14:30:00"
pub fn filtro_hora(val: Value, _args: &[Value]) -> Result<Value, JinjaError> {
    let s = val.as_str().unwrap_or("");
    match parse_iso_time(s) {
        Some((h, m, sec)) => Ok(Value::from(format!("{h:02}:{m:02}:{sec:02}"))),
        None => Ok(val),
    }
}

/// `maiusculas`: "TEXTO EM MAIÚSCULAS"
pub fn filtro_maiusculas(val: Value, _args: &[Value]) -> Result<Value, JinjaError> {
    Ok(Value::from(val.as_str().unwrap_or("").to_uppercase()))
}

/// `minusculas`: "texto em minúsculas"
pub fn filtro_minusculas(val: Value, _args: &[Value]) -> Result<Value, JinjaError> {
    Ok(Value::from(val.as_str().unwrap_or("").to_lowercase()))
}

/// `duracao`: converte segundos (u64) em "1h 23min 05s" ou "23min 05s"
pub fn filtro_duracao(val: Value, _args: &[Value]) -> Result<Value, JinjaError> {
    let secs = match val.as_i64().or_else(|| val.as_str().and_then(|s| s.parse().ok())) {
        Some(v) if v >= 0 => v as u64,
        _ => {
            return Err(JinjaError::new(
                ErrorKind::InvalidOperation,
                "filtro 'duracao' requer um número inteiro de segundos",
            ))
        }
    };
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    let resultado = if h > 0 {
        format!("{h}h {m:02}min {s:02}s")
    } else {
        format!("{m}min {s:02}s")
    };
    Ok(Value::from(resultado))
}

/// `bytes`: converte bytes (u64) em "1,4 GB", "512,0 MB", etc.
pub fn filtro_bytes(val: Value, _args: &[Value]) -> Result<Value, JinjaError> {
    let bytes = match val.as_i64().or_else(|| val.as_str().and_then(|s| s.parse().ok())) {
        Some(v) if v >= 0 => v as u64,
        _ => {
            return Err(JinjaError::new(
                ErrorKind::InvalidOperation,
                "filtro 'bytes' requer um número inteiro de bytes",
            ))
        }
    };
    let resultado = formatar_bytes(bytes);
    Ok(Value::from(resultado))
}

/// `agrupar8`: agrupa o hash em grupos de 8 caracteres separados por espaço.
/// Ex.: "a1b2c3d4 e5f60718 293a4b5c 6d7e8f90"
pub fn filtro_agrupar8(val: Value, _args: &[Value]) -> Result<Value, JinjaError> {
    let s = val.as_str().unwrap_or("");
    let grupos: Vec<String> = s
        .chars()
        .collect::<Vec<_>>()
        .chunks(8)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect();
    Ok(Value::from(grupos.join(" ")))
}

// ─── Funções auxiliares ───────────────────────────────────────────────────────

/// Extrai (year, month, day) de uma string ISO-8601 ou "YYYY-MM-DD".
fn parse_iso_date(s: &str) -> Option<(i32, u8, u8)> {
    // Suporta "2026-11-17T14:30:00..." ou "2026-11-17"
    let date_part = s.get(..10)?;
    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let month: u8 = parts[1].parse().ok()?;
    let day: u8 = parts[2].parse().ok()?;
    if month < 1 || month > 12 || day < 1 || day > 31 {
        return None;
    }
    Some((year, month, day))
}

/// Extrai (hour, minute, second) de uma string ISO-8601 ou "HH:MM:SS".
fn parse_iso_time(s: &str) -> Option<(u8, u8, u8)> {
    // Suporta "2026-11-17T14:30:00..." ou "14:30:00"
    let time_part = if let Some(t_pos) = s.find('T') {
        s.get(t_pos + 1..)?
    } else {
        s
    };
    let time_part = &time_part[..time_part.len().min(8)];
    let parts: Vec<&str> = time_part.split(':').collect();
    if parts.len() < 2 {
        return None;
    }
    let h: u8 = parts[0].parse().ok()?;
    let m: u8 = parts[1].parse().ok()?;
    let sec: u8 = parts.get(2).and_then(|s| s.get(..2)).and_then(|s| s.parse().ok()).unwrap_or(0);
    Some((h, m, sec))
}

fn nome_mes(month: u8) -> &'static str {
    match month {
        1 => "janeiro",
        2 => "fevereiro",
        3 => "março",
        4 => "abril",
        5 => "maio",
        6 => "junho",
        7 => "julho",
        8 => "agosto",
        9 => "setembro",
        10 => "outubro",
        11 => "novembro",
        12 => "dezembro",
        _ => "?",
    }
}

fn formatar_bytes(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    const KB: u64 = 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64).replace('.', ",")
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64).replace('.', ",")
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64).replace('.', ",")
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtro_data_longa_ok() {
        let v = Value::from("2026-11-17T14:30:00-03:00");
        let result = filtro_data_longa(v, &[]).unwrap();
        assert_eq!(result.as_str().unwrap(), "17 de novembro de 2026");
    }

    #[test]
    fn filtro_data_ok() {
        let v = Value::from("2026-11-17T14:30:00");
        let result = filtro_data(v, &[]).unwrap();
        assert_eq!(result.as_str().unwrap(), "17/11/2026");
    }

    #[test]
    fn filtro_hora_ok() {
        let v = Value::from("2026-11-17T14:30:45-03:00");
        let result = filtro_hora(v, &[]).unwrap();
        assert_eq!(result.as_str().unwrap(), "14:30:45");
    }

    #[test]
    fn filtro_duracao_horas() {
        let v = Value::from(5100i64); // 1h 25min 00s
        let result = filtro_duracao(v, &[]).unwrap();
        assert_eq!(result.as_str().unwrap(), "1h 25min 00s");
    }

    #[test]
    fn filtro_duracao_minutos() {
        let v = Value::from(83i64); // 1min 23s
        let result = filtro_duracao(v, &[]).unwrap();
        assert_eq!(result.as_str().unwrap(), "1min 23s");
    }

    #[test]
    fn filtro_bytes_gb() {
        let v = Value::from(1_572_864_000i64);
        let result = filtro_bytes(v, &[]).unwrap();
        assert!(result.as_str().unwrap().contains("GB"));
    }

    #[test]
    fn filtro_agrupar8() {
        let v = Value::from("a1b2c3d4e5f60718");
        let result = filtro_agrupar8(v, &[]).unwrap();
        assert_eq!(result.as_str().unwrap(), "a1b2c3d4 e5f60718");
    }

    #[test]
    fn filtro_agrupar8_impar() {
        let v = Value::from("abcdefghi"); // 9 chars → "abcdefgh i"
        let result = filtro_agrupar8(v, &[]).unwrap();
        assert_eq!(result.as_str().unwrap(), "abcdefgh i");
    }
}
