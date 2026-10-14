//! Native endpoint identity and protected loopback management. Credentials never cross the browser boundary.
pub mod bridge;
pub mod cache;
pub mod central;
pub mod clock;
pub mod credentials;
pub mod install;
pub mod management;
#[cfg(target_os = "macos")]
pub mod owned_process;
pub mod session;
pub mod store;
pub mod sync;

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
    #[error("The selected control plane changed; login explicitly to this server")]
    ServerMismatch,
    #[error("Local identity changed; retry the operation")]
    IdentityChanged,
    #[error("No validated configuration is available; synchronize online")]
    SyncRequired,
    #[error("Encrypted configuration is invalid; synchronize online")]
    InvalidCache,
    #[error("Configuration has expired; synchronize online")]
    CacheExpired,
    #[error("Clock freshness cannot be established; synchronize online")]
    ClockUncertain,
    #[error("Configuration revision or validation time regressed")]
    RevisionRegression,
    #[error(
        "The executable or working directory is missing; install the required program and check its location"
    )]
    MissingExecutable,
    #[error("The local operation was cancelled")]
    Cancelled,
    #[error("The command cannot run; check executable and working-directory permissions")]
    ExecutionDenied,
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
