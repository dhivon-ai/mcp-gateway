//! Versioned AEAD for endpoint snapshots and owner-scoped central vault records.
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};
use zeroize::Zeroizing;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Invalid encrypted format or key configuration")]
    Invalid,
    #[error("The required encryption key is unavailable")]
    MissingKey,
    #[error("Encrypted data integrity check failed")]
    Integrity,
    #[error("Secure randomness is unavailable")]
    Entropy,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Sealed {
    pub format: u32,
    pub key_id: String,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct KeyFile {
    active_key_id: String,
    keys: BTreeMap<String, String>,
}
#[derive(Clone)]
pub struct KeyRing {
    active: String,
    keys: Arc<BTreeMap<String, Zeroizing<[u8; 32]>>>,
}
impl KeyRing {
    pub fn new(active: String, keys: BTreeMap<String, [u8; 32]>) -> Result<Self, CryptoError> {
        if keys.is_empty()
            || keys.len() > 16
            || !keys.contains_key(&active)
            || keys.keys().any(|s| {
                s.is_empty()
                    || s.len() > 64
                    || !s
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
            })
        {
            return Err(CryptoError::Invalid);
        }
        Ok(Self {
            active,
            keys: Arc::new(
                keys.into_iter()
                    .map(|(id, key)| (id, Zeroizing::new(key)))
                    .collect(),
            ),
        })
    }
    pub fn from_json(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() > 16384 {
            return Err(CryptoError::Invalid);
        }
        let file: KeyFile = serde_json::from_slice(bytes).map_err(|_| CryptoError::Invalid)?;
        let mut keys = BTreeMap::new();
        for (id, encoded) in file.keys {
            let bytes = STANDARD.decode(encoded).map_err(|_| CryptoError::Invalid)?;
            let key: [u8; 32] = bytes.try_into().map_err(|_| CryptoError::Invalid)?;
            keys.insert(id, key);
        }
        Self::new(file.active_key_id, keys)
    }
    pub fn generate_key_file(id: &str) -> Result<Vec<u8>, CryptoError> {
        let mut key = [0u8; 32];
        getrandom::fill(&mut key).map_err(|_| CryptoError::Entropy)?;
        Self::new(id.to_string(), BTreeMap::from([(id.to_string(), key)]))?;
        let file = KeyFile {
            active_key_id: id.into(),
            keys: BTreeMap::from([(id.into(), STANDARD.encode(key))]),
        };
        serde_json::to_vec_pretty(&file).map_err(|_| CryptoError::Invalid)
    }
    pub fn active_key_id(&self) -> &str {
        &self.active
    }
    fn aad(id: &str, binding: &[u8]) -> Vec<u8> {
        [
            b"dhivon-mcp-gateway-aead-v1\0".as_slice(),
            id.as_bytes(),
            b"\0",
            binding,
        ]
        .concat()
    }
    pub fn seal(&self, binding: &[u8], plaintext: &[u8]) -> Result<Sealed, CryptoError> {
        if plaintext.len() > 2 * 1024 * 1024 || binding.len() > 4096 {
            return Err(CryptoError::Invalid);
        }
        let key = self.keys.get(&self.active).ok_or(CryptoError::MissingKey)?;
        let cipher = Aes256Gcm::new_from_slice(key.as_ref()).map_err(|_| CryptoError::Invalid)?;
        let mut nonce = [0u8; 12];
        getrandom::fill(&mut nonce).map_err(|_| CryptoError::Entropy)?;
        let ciphertext = cipher
            .encrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: plaintext,
                    aad: &Self::aad(&self.active, binding),
                },
            )
            .map_err(|_| CryptoError::Integrity)?;
        Ok(Sealed {
            format: 1,
            key_id: self.active.clone(),
            nonce: nonce.to_vec(),
            ciphertext,
        })
    }
    pub fn open(&self, binding: &[u8], sealed: &Sealed) -> Result<Vec<u8>, CryptoError> {
        if sealed.format != 1
            || sealed.nonce.len() != 12
            || sealed.ciphertext.len() > 2 * 1024 * 1024 + 16
            || binding.len() > 4096
        {
            return Err(CryptoError::Invalid);
        }
        let key = self
            .keys
            .get(&sealed.key_id)
            .ok_or(CryptoError::MissingKey)?;
        let cipher = Aes256Gcm::new_from_slice(key.as_ref()).map_err(|_| CryptoError::Invalid)?;
        let nonce: [u8; 12] = sealed
            .nonce
            .as_slice()
            .try_into()
            .map_err(|_| CryptoError::Invalid)?;
        cipher
            .decrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: &sealed.ciphertext,
                    aad: &Self::aad(&sealed.key_id, binding),
                },
            )
            .map_err(|_| CryptoError::Integrity)
    }
}
