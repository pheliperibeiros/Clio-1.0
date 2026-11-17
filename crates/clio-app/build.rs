//! build.rs do clio-app — emite ícone no .exe (Windows) e metadados de versão.
use std::env;

fn main() {
    // Embutir ícone no executável Windows
    #[cfg(target_os = "windows")]
    embed_icon_windows();

    // Emitir hash do commit como variável de ambiente de compilação
    emit_build_hash();

    // Re-executar se os assets mudarem
    println!("cargo:rerun-if-changed=../../assets/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");
}

#[cfg(target_os = "windows")]
fn embed_icon_windows() {
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../../assets/icon.ico");
    res.set("FileDescription", "Clio — Gravador de Oitivas e Audiências");
    res.set("ProductName", "Clio");
    res.set(
        "CompanyName",
        "Ministério Público do Estado do Tocantins — GAECO",
    );
    res.set("LegalCopyright", "MPTO — uso institucional");
    res.set(
        "FileVersion",
        env::var("CARGO_PKG_VERSION").unwrap_or_default().as_str(),
    );
    res.set(
        "ProductVersion",
        env::var("CARGO_PKG_VERSION").unwrap_or_default().as_str(),
    );
    if let Err(e) = res.compile() {
        eprintln!("Aviso: não foi possível compilar recurso do ícone: {e}");
    }
}

fn emit_build_hash() {
    // Tentar ler o hash do commit via git
    let hash = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "desconhecido".to_string());

    println!("cargo:rustc-env=CLIO_BUILD_HASH={hash}");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
}
