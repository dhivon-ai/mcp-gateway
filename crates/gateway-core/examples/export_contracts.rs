use gateway_core::*;
use ts_rs::{Config, TS};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::args()
        .nth(1)
        .ok_or("usage: export_contracts <output-directory>")?;
    let config = Config::new().with_large_int("number").with_out_dir(dir);
    ConfigSnapshot::export_all(&config)?;
    GroupInfo::export_all(&config)?;
    UserCreate::export_all(&config)?;
    UserEdit::export_all(&config)?;
    PasswordReset::export_all(&config)?;
    GroupWrite::export_all(&config)?;
    Assignment::export_all(&config)?;
    CatalogWrite::export_all(&config)?;
    AssignmentWrite::export_all(&config)?;
    RevisionInput::export_all(&config)?;
    RevisionInfo::export_all(&config)?;
    AdminConfiguration::export_all(&config)?;
    SecretMetadata::export_all(&config)?;
    LoginRequest::export_all(&config)?;
    SettingsUpdate::export_all(&config)?;
    SecretWrite::export_all(&config)?;
    SecretRename::export_all(&config)?;
    VersionInput::export_all(&config)?;
    OAuthStatus::export_all(&config)?;
    InventoryReport::export_all(&config)?;
    UiContext::export_all(&config)?;
    ApiError::export_all(&config)?;
    Ok(())
}
