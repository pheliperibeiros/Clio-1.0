//! Cadeia de hashes incrementais.
//!
//! A cada bloco de N MiB gravados:
//!   H_0 = SHA256(bloco_0)
//!   H_i = SHA256(H_{i-1} || SHA256(bloco_i))
//!
//! O hash final do arquivo calculado de forma incremental (lendo em blocos de 1 MiB)
//! coincide com o SHA256 do arquivo completo.

use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};
use crate::error::CustodyResult;

/// Tamanho de cada bloco da cadeia: 4 MiB.
pub const CHAIN_BLOCK_SIZE: usize = 4 * 1024 * 1024;

/// Tamanho do bloco de leitura para o hash incremental do arquivo: 1 MiB.
pub const READ_BLOCK_SIZE: usize = 1024 * 1024;

/// Um bloco da cadeia de hashes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainBlock {
    /// Índice do bloco (0-based).
    pub index: usize,
    /// Offset do início deste bloco no arquivo, em bytes.
    pub offset_bytes: u64,
    /// SHA-256 deste bloco isolado (hex).
    pub block_hash: String,
    /// Hash encadeado H_i = SHA256(H_{i-1} || SHA256(bloco_i)) (hex).
    /// Para i=0, H_0 = SHA256(bloco_0).
    pub chain_hash: String,
}

/// Acumula hashes em tempo real conforme o arquivo cresce.
/// Uso: chamar `feed()` a cada byte gravado; chamar `checkpoint()` a cada `CHAIN_BLOCK_SIZE`.
pub struct HashChain {
    blocks: Vec<ChainBlock>,
    block_hasher: Sha256,
    file_hasher: Sha256,
    bytes_no_bloco_atual: usize,
    total_bytes: u64,
    ultimo_chain_hash: Option<[u8; 32]>,
}

impl HashChain {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            block_hasher: Sha256::new(),
            file_hasher: Sha256::new(),
            bytes_no_bloco_atual: 0,
            total_bytes: 0,
            ultimo_chain_hash: None,
        }
    }

    /// Alimenta dados. Chama automaticamente `checkpoint()` quando o bloco enche.
    pub fn feed(&mut self, data: &[u8]) {
        self.file_hasher.update(data);
        let mut rest = data;
        while !rest.is_empty() {
            let space = CHAIN_BLOCK_SIZE - self.bytes_no_bloco_atual;
            let take = rest.len().min(space);
            self.block_hasher.update(&rest[..take]);
            self.bytes_no_bloco_atual += take;
            self.total_bytes += take as u64;
            rest = &rest[take..];
            if self.bytes_no_bloco_atual >= CHAIN_BLOCK_SIZE {
                self.flush_block();
            }
        }
    }

    fn flush_block(&mut self) {
        let block_hash_bytes: [u8; 32] = self
            .block_hasher
            .clone()
            .finalize()
            .into();
        self.block_hasher = Sha256::new(); // reset

        let chain_hash_bytes = if let Some(prev) = self.ultimo_chain_hash {
            let mut h = Sha256::new();
            h.update(prev);
            h.update(block_hash_bytes);
            h.finalize().into()
        } else {
            block_hash_bytes
        };

        let offset = self.total_bytes - self.bytes_no_bloco_atual as u64;
        let idx = self.blocks.len();
        self.blocks.push(ChainBlock {
            index: idx,
            offset_bytes: offset,
            block_hash: hex::encode(block_hash_bytes),
            chain_hash: hex::encode(chain_hash_bytes),
        });
        self.ultimo_chain_hash = Some(chain_hash_bytes);
        self.bytes_no_bloco_atual = 0;
    }

    /// Finaliza o bloco parcial atual (se houver) e retorna o hash final do arquivo.
    /// Após chamar este método, o `HashChain` não deve ser mais alimentado.
    pub fn finalize(mut self) -> ([u8; 32], Vec<ChainBlock>) {
        if self.bytes_no_bloco_atual > 0 {
            self.flush_block();
        }
        let file_hash: [u8; 32] = self.file_hasher.finalize().into();
        (file_hash, self.blocks)
    }

    pub fn blocks(&self) -> &[ChainBlock] {
        &self.blocks
    }

    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }
}

impl Default for HashChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Calcula o SHA-256 de um arquivo inteiro em blocos de 1 MiB,
/// sem carregar o arquivo em memória.
/// Reporta progresso via callback `on_progress(bytes_lidos, total_bytes)`.
pub fn hash_arquivo<P, F>(
    path: P,
    on_progress: F,
) -> CustodyResult<[u8; 32]>
where
    P: AsRef<std::path::Path>,
    F: Fn(u64, u64),
{
    use std::io::Read;

    let path = path.as_ref();
    let total = std::fs::metadata(path)?.len();
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; READ_BLOCK_SIZE];
    let mut lidos = 0u64;

    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        lidos += n as u64;
        on_progress(lidos, total);
    }

    let hash: [u8; 32] = hasher.finalize().into();
    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vetor conhecido: SHA256("") = e3b0c44298fc1c14...
    #[test]
    fn hash_vazio() {
        let mut h = Sha256::new();
        let result: [u8; 32] = h.finalize().into();
        assert_eq!(
            hex::encode(result),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn hash_chain_pequeno() {
        let mut chain = HashChain::new();
        chain.feed(b"hello world");
        let (hash, blocks) = chain.finalize();
        // Deve ter 1 bloco (dados < CHAIN_BLOCK_SIZE)
        assert_eq!(blocks.len(), 1);
        // O hash do arquivo deve ser SHA256("hello world")
        let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe04294e576db3a0dc7f7deec32d";
        // Nota: SHA256("hello world") real:
        // b94d27b9934d3e08a52e52d7da7dabfac484efe04294e576db3a0dc7f7deec32d
        // Vamos verificar com sha2 diretamente
        let mut h = Sha256::new();
        h.update(b"hello world");
        let direct: [u8; 32] = h.finalize().into();
        assert_eq!(hash, direct, "hash do arquivo deve coincidir com SHA256 direto");
    }

    #[test]
    fn hash_chain_multiplos_blocos() {
        let mut chain = HashChain::new();
        // Alimentar exatamente 2 blocos + resto
        let bloco = vec![0u8; CHAIN_BLOCK_SIZE];
        chain.feed(&bloco);
        chain.feed(&bloco);
        chain.feed(b"final");
        let (_, blocks) = chain.finalize();
        assert_eq!(blocks.len(), 3, "deve ter 3 blocos: 2 cheios + 1 parcial");
    }

    #[test]
    fn hash_arquivo_em_disco() {
        use std::io::Write;
        let path = std::env::temp_dir().join("clio_hash_test.bin");
        let data = b"dados de teste para hash";
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(data).unwrap();
        }

        let hash = hash_arquivo(&path, |_, _| {}).unwrap();

        let mut h = Sha256::new();
        h.update(data);
        let expected: [u8; 32] = h.finalize().into();
        assert_eq!(hash, expected);

        std::fs::remove_file(&path).unwrap();
    }
}
