//! Migrações de esquema de configuração.
//!
//! Cada migração recebe o `toml::Value` cru e retorna a versão atualizada.

use crate::error::{ConfigError, ConfigResult};
use crate::schema::SCHEMA_VERSION;

/// Aplica todas as migrações necessárias até a versão atual.
pub fn migrar(mut valor: toml::Value, versao_encontrada: u32) -> ConfigResult<toml::Value> {
    let mut v = versao_encontrada;

    while v < SCHEMA_VERSION {
        valor = match v {
            1 => migrar_v1_para_v2(valor)?,
            2 => migrar_v2_para_v3(valor)?,
            other => {
                return Err(ConfigError::UnknownSchemaVersion(other));
            }
        };
        v += 1;
    }

    Ok(valor)
}

/// v1 → v2: Adicionado campo `audio.modo` (padrão: "faixas_separadas").
fn migrar_v1_para_v2(mut valor: toml::Value) -> ConfigResult<toml::Value> {
    if let Some(audio) = valor.get_mut("audio").and_then(|v| v.as_table_mut()) {
        audio
            .entry("modo")
            .or_insert_with(|| toml::Value::String("faixas_separadas".to_string()));
    }
    if let Some(root) = valor.as_table_mut() {
        root.insert("schema_version".to_string(), toml::Value::Integer(2));
    }
    Ok(valor)
}

/// v2 → v3: Adicionado campo `identidade_visual.cabecalho_ativo` (padrão: true)
/// e `emissao.local` (padrão: "Palmas").
fn migrar_v2_para_v3(mut valor: toml::Value) -> ConfigResult<toml::Value> {
    // identidade_visual.cabecalho_ativo
    if let Some(iv) = valor
        .get_mut("identidade_visual")
        .and_then(|v| v.as_table_mut())
    {
        iv.entry("cabecalho_ativo")
            .or_insert_with(|| toml::Value::Boolean(true));
    }
    // emissao.local
    {
        let root = valor
            .as_table_mut()
            .ok_or_else(|| ConfigError::MigrationFailed {
                from: 2,
                to: 3,
                reason: "raiz não é tabela TOML".to_string(),
            })?;
        root.entry("emissao").or_insert_with(|| {
            let mut t = toml::map::Map::new();
            t.insert(
                "local".to_string(),
                toml::Value::String("Palmas".to_string()),
            );
            toml::Value::Table(t)
        });
        root.insert("schema_version".to_string(), toml::Value::Integer(3));
    }
    Ok(valor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migracao_v1_para_v3() {
        // Simula um config.toml da v1 mínimo
        let toml_v1 = r#"
schema_version = 1
[audio]
mic1_rotulo = "Autoridade"
"#;
        let valor: toml::Value = toml::from_str(toml_v1).unwrap();
        let migrado = migrar(valor, 1).unwrap();
        let versao = migrado
            .get("schema_version")
            .and_then(|v| v.as_integer())
            .unwrap();
        assert_eq!(versao, 3);

        // Verificar que modo foi adicionado
        let modo = migrado
            .get("audio")
            .and_then(|a| a.get("modo"))
            .and_then(|m| m.as_str())
            .unwrap();
        assert_eq!(modo, "faixas_separadas");
    }

    #[test]
    fn migracao_v2_para_v3() {
        let toml_v2 = r#"
schema_version = 2
[audio]
modo = "faixas_separadas"
"#;
        let valor: toml::Value = toml::from_str(toml_v2).unwrap();
        let migrado = migrar(valor, 2).unwrap();
        let versao = migrado
            .get("schema_version")
            .and_then(|v| v.as_integer())
            .unwrap();
        assert_eq!(versao, 3);
    }

    #[test]
    fn versao_atual_sem_migracao() {
        let toml_atual = format!("schema_version = {SCHEMA_VERSION}\n");
        let valor: toml::Value = toml::from_str(&toml_atual).unwrap();
        // Não deve alterar nada
        let migrado = migrar(valor, SCHEMA_VERSION).unwrap();
        let versao = migrado
            .get("schema_version")
            .and_then(|v| v.as_integer())
            .unwrap();
        assert_eq!(versao as u32, SCHEMA_VERSION);
    }
}
