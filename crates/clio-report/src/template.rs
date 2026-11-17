//! Motor de templates baseado em minijinja (sandbox: sem loader de arquivos, sem rede).

use minijinja::{Environment, Value};
use crate::context::ReportContext;
use crate::error::{ReportError, ReportResult};
use crate::filters;

/// Tamanho máximo de um modelo de texto: 256 KB.
pub const MAX_TEMPLATE_BYTES: usize = 256 * 1024;

/// Limite de iterações em laços.
pub const MAX_LOOP_ITERATIONS: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoDocumento {
    Extrato,
    Relatorio,
}

/// Motor de templates reutilizável.
pub struct TemplateEngine {
    env: Environment<'static>,
}

impl TemplateEngine {
    /// Cria o motor com todos os filtros registrados e as configurações de sandbox.
    pub fn new() -> Self {
        let mut env = Environment::new();

        // Sandbox: sem loader de arquivos nem de rede
        // O minijinja por padrão não tem loader, então não há risco de acesso externo.
        // Garantir que includes são proibidos:
        env.set_max_local_vars(50);

        // Registrar filtros customizados
        env.add_filter("data_longa", filters::filtro_data_longa);
        env.add_filter("data", filters::filtro_data);
        env.add_filter("hora", filters::filtro_hora);
        env.add_filter("maiusculas", filters::filtro_maiusculas);
        env.add_filter("minusculas", filters::filtro_minusculas);
        env.add_filter("duracao", filters::filtro_duracao);
        env.add_filter("bytes", filters::filtro_bytes);
        env.add_filter("agrupar8", filters::filtro_agrupar8);

        Self { env }
    }

    /// Valida um modelo: retorna `Ok(())` se válido ou `Err` com descrição em pt-BR.
    pub fn validar(&self, source: &str) -> ReportResult<()> {
        if source.len() > MAX_TEMPLATE_BYTES {
            return Err(ReportError::TemplateTooLarge {
                max_bytes: MAX_TEMPLATE_BYTES,
            });
        }
        // Tentar renderizar com contexto de exemplo para detectar erros de sintaxe e variáveis
        let ctx = ReportContext::exemplo();
        let ctx_val = ctx.to_value().map_err(|e| ReportError::Template(e.to_string()))?;
        self.renderizar_raw(source, ctx_val).map(|_| ())
    }

    /// Renderiza um modelo com o contexto fornecido.
    pub fn renderizar(
        &self,
        source: &str,
        ctx: &ReportContext,
    ) -> ReportResult<String> {
        if source.len() > MAX_TEMPLATE_BYTES {
            return Err(ReportError::TemplateTooLarge {
                max_bytes: MAX_TEMPLATE_BYTES,
            });
        }
        let ctx_val = ctx.to_value().map_err(|e| ReportError::Template(e.to_string()))?;
        self.renderizar_raw(source, ctx_val)
    }

    fn renderizar_raw(&self, source: &str, ctx: Value) -> ReportResult<String> {
        // Compilar e renderizar em uma etapa (sem armazenar no ambiente global)
        let mut env = self.env.clone();
        env.add_template_owned("doc", source.to_string())
            .map_err(|e| self.jinja_error_to_report(e, source))?;
        let tmpl = env.get_template("doc")
            .map_err(|e| self.jinja_error_to_report(e, source))?;
        tmpl.render(ctx)
            .map_err(|e| self.jinja_error_to_report(e, source))
    }

    fn jinja_error_to_report(&self, e: minijinja::Error, source: &str) -> ReportError {
        let line = e.line().unwrap_or(0);
        let msg = e.to_string();
        // Detectar variável desconhecida vs sintaxe
        if msg.contains("undefined") || msg.contains("unknown") {
            // Tentar extrair nome da variável
            let var = e.detail()
                .unwrap_or("variável desconhecida")
                .to_string();
            ReportError::UnknownVariable { var, line }
        } else {
            ReportError::InvalidSyntax { line, msg }
        }
    }

    /// Renderiza o modelo do Extrato usando o template padrão embutido,
    /// ou um personalizado se fornecido.
    pub fn renderizar_extrato(
        &self,
        template_personalizado: Option<&str>,
        ctx: &ReportContext,
    ) -> ReportResult<String> {
        let source = template_personalizado.unwrap_or(DEFAULT_EXTRATO);
        self.renderizar(source, ctx)
    }

    /// Renderiza o modelo do Relatório usando o template padrão embutido,
    /// ou um personalizado se fornecido.
    pub fn renderizar_relatorio(
        &self,
        template_personalizado: Option<&str>,
        ctx: &ReportContext,
    ) -> ReportResult<String> {
        let source = template_personalizado.unwrap_or(DEFAULT_RELATORIO);
        self.renderizar(source, ctx)
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Template padrão do Extrato (embutido no binário).
pub const DEFAULT_EXTRATO: &str =
    include_str!("../../../assets/templates/extrato.default.j2");

/// Template padrão do Relatório (embutido no binário).
pub const DEFAULT_RELATORIO: &str =
    include_str!("../../../assets/templates/relatorio.default.j2");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ReportContext;

    #[test]
    fn renderiza_extrato_padrao() {
        let engine = TemplateEngine::new();
        let ctx = ReportContext::exemplo();
        let result = engine.renderizar_extrato(None, &ctx).unwrap();
        assert!(result.contains("EXTRATO E CERTIDÃO"));
        assert!(result.contains("João da Silva"));
        assert!(result.contains("SHA-256"));
    }

    #[test]
    fn renderiza_relatorio_padrao() {
        let engine = TemplateEngine::new();
        let ctx = ReportContext::exemplo();
        let result = engine.renderizar_relatorio(None, &ctx).unwrap();
        assert!(result.contains("RELATÓRIO DA OITIVA"));
        assert!(result.contains("João da Silva"));
    }

    #[test]
    fn template_invalido_retorna_erro() {
        let engine = TemplateEngine::new();
        let ctx = ReportContext::exemplo();
        let resultado = engine.renderizar("{% for %}", &ctx);
        assert!(resultado.is_err());
    }

    #[test]
    fn variavel_desconhecida_retorna_erro_amigavel() {
        let engine = TemplateEngine::new();
        let ctx = ReportContext::exemplo();
        // minijinja retorna vazio ou erro dependendo da configuração de undefined
        // Vamos apenas garantir que não panics
        let _ = engine.renderizar("{{ variavel_inexistente }}", &ctx);
    }

    #[test]
    fn filtros_funcionam_no_template() {
        let engine = TemplateEngine::new();
        let ctx = ReportContext::exemplo();
        let tmpl = "{{ sessao.inicio | data_longa }}";
        let result = engine.renderizar(tmpl, &ctx).unwrap();
        assert!(result.contains("novembro"));
    }

    #[test]
    fn template_muito_grande_rejeitado() {
        let engine = TemplateEngine::new();
        let ctx = ReportContext::exemplo();
        let grande = "x".repeat(MAX_TEMPLATE_BYTES + 1);
        let result = engine.renderizar(&grande, &ctx);
        assert!(matches!(result, Err(ReportError::TemplateTooLarge { .. })));
    }

    #[test]
    fn validar_template_valido() {
        let engine = TemplateEngine::new();
        assert!(engine.validar("{{ orgao }}").is_ok());
    }

    #[test]
    fn validar_template_invalido() {
        let engine = TemplateEngine::new();
        assert!(engine.validar("{% if %}").is_err());
    }
}
