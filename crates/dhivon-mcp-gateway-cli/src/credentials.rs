use crate::{EndpointError, Result};
use zeroize::Zeroizing;

/// Credential ownership stays typed; callers cannot overwrite the central session as an upstream token.
#[derive(Clone, Copy)]
pub enum CredentialKind {
    CacheKey,
    Upstream { mcp_id: uuid::Uuid, version: u32 },
}
impl CredentialKind {
    pub(crate) fn label(self) -> Result<String> {
        match self {
            Self::CacheKey => Ok("cache-key".into()),
            Self::Upstream { mcp_id, version } if version > 0 => {
                Ok(format!("upstream:{mcp_id}:{version}"))
            }
            _ => Err(EndpointError::Invalid),
        }
    }
}

/// Small safe seam for deterministic tests; production has no plaintext fallback.
pub trait CredentialStore: Send + Sync {
    fn get(&self, account: &str) -> Result<Option<Zeroizing<Vec<u8>>>>;
    fn set(&self, account: &str, value: &[u8]) -> Result<()>;
    fn delete(&self, account: &str) -> Result<()>;
}
pub struct MacKeychain {
    pub(crate) service: String,
}
impl MacKeychain {
    pub fn new(service: String) -> Self {
        Self { service }
    }
    /// Explicit fixture state uses the real default Keychain with an isolated deterministic service.
    pub fn fixture(root: &std::path::Path) -> Result<Self> {
        use base64::Engine;
        use sha2::{Digest, Sha256};
        if !root.is_absolute() {
            return Err(EndpointError::Storage);
        }
        crate::store::check_private(root, true)?;
        let canonical = root.canonicalize().map_err(|_| EndpointError::Storage)?;
        #[cfg(target_os = "macos")]
        if let Some(home) = std::env::var_os("HOME") {
            let production = std::path::PathBuf::from(home)
                .join("Library/Application Support/com.dhivon.mcp-gateway");
            if canonical.starts_with(production.canonicalize().unwrap_or(production)) {
                return Err(EndpointError::Invalid);
            }
        }
        let canonical = canonical.to_str().ok_or(EndpointError::Storage)?;
        let digest = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(Sha256::digest(canonical.as_bytes()));
        Ok(Self::new(format!(
            "com.dhivon.mcp-gateway.fixture.{digest}"
        )))
    }
    pub fn production() -> Self {
        Self::new("com.dhivon.mcp-gateway.endpoint.v1".into())
    }
}
#[cfg(target_os = "macos")]
impl CredentialStore for MacKeychain {
    fn get(&self, account: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        match security_framework::passwords::get_generic_password(&self.service, account) {
            Ok(v) => Ok(Some(Zeroizing::new(v))),
            Err(e) if e.code() == -25300 => Ok(None),
            Err(_) => Err(EndpointError::Keychain),
        }
    }
    fn set(&self, account: &str, value: &[u8]) -> Result<()> {
        security_framework::passwords::set_generic_password(&self.service, account, value)
            .map_err(|_| EndpointError::Keychain)
    }
    fn delete(&self, account: &str) -> Result<()> {
        match security_framework::passwords::delete_generic_password(&self.service, account) {
            Ok(()) => Ok(()),
            Err(e) if e.code() == -25300 => Ok(()),
            Err(_) => Err(EndpointError::Keychain),
        }
    }
}
#[cfg(not(target_os = "macos"))]
impl CredentialStore for MacKeychain {
    fn get(&self, _: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        let _ = &self.service;
        Err(EndpointError::Unsupported)
    }
    fn set(&self, _: &str, _: &[u8]) -> Result<()> {
        Err(EndpointError::Unsupported)
    }
    fn delete(&self, _: &str) -> Result<()> {
        Err(EndpointError::Unsupported)
    }
}
