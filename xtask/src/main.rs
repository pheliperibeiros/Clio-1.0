//! Automação de tarefas de build, licenças, SBOM e empacotamento MSI para o Clio.
#![forbid(unsafe_code)]

use std::env;
use std::process::{Command, ExitStatus};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let task = args.first().map(|s| s.as_str()).unwrap_or("help");

    match task {
        "licenses" => task_licenses(),
        "sbom" => task_sbom(),
        "package" | "dist" => task_package(),
        "ci" => task_ci(),
        _ => print_help(),
    }
}

fn print_help() {
    println!("Clio xtask — Ferramentas de Manutenção e Empacotamento");
    println!("Uso: cargo xtask <comando>");
    println!();
    println!("Comandos:");
    println!("  licenses   Verifica conformidade de licenças via cargo-deny");
    println!("  sbom       Gera Software Bill of Materials (SBOM) via cargo-cyclonedx");
    println!("  package    Compila binário em modo release e gera instalador MSI (WiX)");
    println!("  ci         Executa suite completa de validação local (fmt, clippy, test, deny)");
}

fn run_cmd(cmd: &str, args: &[&str]) -> ExitStatus {
    println!("==> Executando: {} {}", cmd, args.join(" "));
    let status = Command::new(cmd)
        .args(args)
        .status()
        .unwrap_or_else(|e| panic!("Falha ao executar {cmd}: {e}"));

    if !status.success() {
        eprintln!("Erro: comando retornou código {:?}", status.code());
    }
    status
}

fn task_licenses() {
    println!("--- Verificando licenças com cargo-deny ---");
    let status = run_cmd("cargo", &["deny", "check", "licenses"]);
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
}

fn task_sbom() {
    println!("--- Gerando SBOM com cargo-cyclonedx ---");
    let status = run_cmd(
        "cargo",
        &[
            "cyclonedx",
            "--format",
            "json",
            "--output-pattern",
            "target/clio-sbom.json",
        ],
    );
    if !status.success() {
        eprintln!(
            "Aviso: certifique-se de que cargo-cyclonedx está instalado: cargo install cargo-cyclonedx"
        );
        std::process::exit(status.code().unwrap_or(1));
    }
    println!("SBOM gerado com sucesso em target/clio-sbom.json");
}

fn task_package() {
    println!("--- Compilando Clio em modo Release ---");
    let build_status = run_cmd(
        "cargo",
        &["build", "--release", "-p", "clio-app", "--locked"],
    );
    if !build_status.success() {
        std::process::exit(build_status.code().unwrap_or(1));
    }

    println!("--- Empacotando Instalador MSI com WiX ---");
    // WiX toolset v4/v5 utiliza comando `wix build`
    let wix_status = run_cmd(
        "wix",
        &["build", "installer/clio.wxs", "-o", "target/Clio-Setup.msi"],
    );
    if !wix_status.success() {
        eprintln!("Aviso: WiX Toolset pode não estar no PATH. Verifique a instalação do WiX.");
        std::process::exit(wix_status.code().unwrap_or(1));
    }
    println!("Instalador MSI gerado com sucesso em target/Clio-Setup.msi");
}

fn task_ci() {
    println!("--- Executando Validação Local CI ---");
    let steps = [
        ("cargo", vec!["fmt", "--check"]),
        (
            "cargo",
            vec!["clippy", "--workspace", "--", "-D", "warnings"],
        ),
        ("cargo", vec!["test", "--workspace", "--locked"]),
        ("cargo", vec!["deny", "check"]),
    ];

    for (cmd, args) in steps {
        let status = run_cmd(cmd, &args);
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
    }
    println!("Todos os passos do CI concluídos com sucesso!");
}
