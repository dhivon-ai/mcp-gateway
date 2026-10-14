//! Native endpoint identity and protected loopback management. Credentials never cross the browser boundary.
pub mod central;
pub mod credentials;
pub mod management;
pub mod session;
pub mod store;

#[derive(Debug, thiserror::Error)]
pub enum EndpointError {
    #[error("Invalid endpoint input or protected state")]
    Invalid,
    #[error("Protected local storage is unavailable; check owner-only permissions")]
    Storage,
    #[error("macOS Keychain is unavailable; unlock it and retry")]
    Keychain,
    #[error("Login is required")]
    LoginRequired,
    #[error("The central session was rejected; login again")]
    Rejected,
    #[error("The control plane is unreachable; retry when online")]
    Offline,
    #[error("The control plane returned an invalid response")]
    Response,
    #[error("Local identity changed; retry the operation")]
    IdentityChanged,
    #[error("This endpoint operation is unsupported on this platform")]
    Unsupported,
}
pub type Result<T> = std::result::Result<T, EndpointError>;

pub fn epoch() -> Result<i64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .ok_or(EndpointError::Invalid)
}
