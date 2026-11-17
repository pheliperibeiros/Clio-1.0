//! Selo local Ed25519 — geração de par de chaves, assinatura e verificação.
//!
//! # Limitações documentadas
//! O selo local detecta adulteração do manifesto por quem **não tem** a chave privada
//! armazenada neste computador. Ele NÃO:
//! - Equivale a assinatura ICP-Brasil nem a carimbo do tempo de terceiro.
//! - Protege contra um operador com acesso físico ao mesmo computador.
//! - Garante não-repúdio perante um tribunal sem cadeia de custódia adicional.
//! A impressão digital da chave pública aparece no Extrato como registro informativo.

use ed25519_dalek::{SigningKey, VerifyingKey, Signer, Verifier, Signature};
use crate::error::{CustodyError, CustodyResult};
use crate::manifest::Manifest;

/// Nome da entrada no chaveiro do sistema.
const KEYRING_SERVICE: &str = "br.mp.to.gaeco.clio";
const KEYRING_USER: &str = "selo-local-ed25519";

pub type SealResult<T> = CustodyResult<T>;

/// Gerenciador do selo local.
pub struct LocalSeal {
    signing_key: SigningKey,
    fingerprint: String,
}

impl LocalSeal {
    /// Carrega a chave existente do chaveiro ou gera uma nova no primeiro uso.
    pub fn carregar_ou_criar() -> SealResult<Self> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
            .map_err(|e| CustodyError::KeyNotFound(e.to_string()))?;

        let raw_bytes = match entry.get_password() {
            Ok(hex_str) => {
                // Chave existente
                hex::decode(hex_str.trim())
                    .map_err(|e| CustodyError::Signature(format!("Chave inválida no chaveiro: {e}")))?
            }
            Err(_) => {
                // Primeira execução: gerar nova chave
                let key = SigningKey::generate(&mut rand::rngs::OsRng);
                let bytes = key.to_bytes();
                entry
                    .set_password(&hex::encode(bytes))
                    .map_err(|e| CustodyError::KeyNotFound(format!("Não foi possível salvar no chaveiro: {e}")))?;
                bytes.to_vec()
            }
        };

        let bytes: [u8; 32] = raw_bytes
            .try_into()
            .map_err(|_| CustodyError::Signature("Comprimento da chave inválido".to_string()))?;
        let signing_key = SigningKey::from_bytes(&bytes);
        let fingerprint = Self::compute_fingerprint(&signing_key.verifying_key());

        Ok(Self { signing_key, fingerprint })
    }

    /// Impressão digital: hex dos primeiros 8 bytes da chave pública.
    fn compute_fingerprint(vk: &VerifyingKey) -> String {
        hex::encode(&vk.as_bytes()[..8])
    }

    /// Impressão digital da chave pública deste selo.
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// Assina o manifesto e preenche os campos `assinatura_ed25519` e `selo_fingerprint`.
    pub fn assinar(&self, manifesto: &mut Manifest) -> SealResult<()> {
        let bytes = manifesto.bytes_para_assinar()?;
        let signature: Signature = self.signing_key.sign(&bytes);
        manifesto.assinatura_ed25519 = Some(hex::encode(signature.to_bytes()));
        manifesto.selo_fingerprint = Some(self.fingerprint.clone());
        Ok(())
    }

    /// Verifica a assinatura de um manifesto já selado.
    pub fn verificar(manifesto: &Manifest, chave_publica_hex: &str) -> SealResult<bool> {
        let sig_hex = manifesto
            .assinatura_ed25519
            .as_deref()
            .ok_or_else(|| CustodyError::Signature("Manifesto não contém assinatura".to_string()))?;

        let sig_bytes = hex::decode(sig_hex)
            .map_err(|e| CustodyError::Signature(format!("Assinatura inválida: {e}")))?;
        let sig_arr: [u8; 64] = sig_bytes
            .try_into()
            .map_err(|_| CustodyError::Signature("Comprimento da assinatura inválido".to_string()))?;
        let signature = Signature::from_bytes(&sig_arr);

        let pk_bytes = hex::decode(chave_publica_hex)
            .map_err(|e| CustodyError::Signature(format!("Chave pública inválida: {e}")))?;
        let pk_arr: [u8; 32] = pk_bytes
            .try_into()
            .map_err(|_| CustodyError::Signature("Comprimento da chave pública inválido".to_string()))?;
        let verifying_key = VerifyingKey::from_bytes(&pk_arr)
            .map_err(|e| CustodyError::Signature(e.to_string()))?;

        let bytes_a_verificar = manifesto.bytes_para_assinar()?;
        Ok(verifying_key.verify(&bytes_a_verificar, &signature).is_ok())
    }
}

#[cfg(test)]
mod tests {
    // Testes de integração do selo requerem acesso ao chaveiro do sistema.
    // Para CI headless, testar apenas a geração em memória.
    use super::*;
    use ed25519_dalek::SigningKey;

    #[test]
    fn fingerprint_deterministica() {
        let bytes = [0u8; 32];
        let key = SigningKey::from_bytes(&bytes);
        let fp = LocalSeal::compute_fingerprint(&key.verifying_key());
        assert_eq!(fp.len(), 16, "fingerprint deve ter 16 hex chars (8 bytes)");
    }
}
