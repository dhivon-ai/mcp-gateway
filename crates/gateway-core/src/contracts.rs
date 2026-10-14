use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Admin,
    User,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub role: Role,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GroupInfo {
    pub id: String,
    pub name: String,
    pub member_ids: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UserCreate {
    pub expected_revision: u32,
    pub username: String,
    pub password: String,
    pub role: Role,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UserEdit {
    pub expected_revision: u32,
    pub role: Role,
    pub active: bool,
}

#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PasswordReset {
    pub expected_revision: u32,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GroupWrite {
    pub expected_revision: u32,
    pub name: String,
    pub member_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Transport {
    Stdio { command: CommandSpec },
    Http { url: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AuthMode {
    None {},
    Bearer {},
    #[serde(rename = "oauth")]
    OAuth {
        #[serde(rename = "clientId")]
        client_id: String,
        scopes: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "mode", content = "tools", rename_all = "snake_case")]
pub enum ToolPolicy {
    Allowlist(Vec<String>),
    Denylist(Vec<String>),
}

impl Default for ToolPolicy {
    fn default() -> Self {
        Self::Allowlist(Vec::new())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub transport: Transport,
    pub install: Option<CommandSpec>,
    pub auth: AuthMode,
    pub tool_policy: ToolPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GrantTarget {
    All {},
    User { id: String },
    Group { id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Assignment {
    pub id: String,
    pub mcp_id: String,
    pub target: GrantTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogWrite {
    pub expected_revision: u32,
    pub definition: McpDefinition,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssignmentWrite {
    pub expected_revision: u32,
    pub mcp_id: String,
    pub target: GrantTarget,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevisionInput {
    pub expected_revision: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevisionInfo {
    pub revision: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolInventory {
    pub mcp_id: String,
    pub names: Vec<String>,
    pub observed_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdminConfiguration {
    pub revision: u32,
    pub mcps: Vec<McpDefinition>,
    pub assignments: Vec<Assignment>,
    pub inventory: Vec<ToolInventory>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretRef {
    pub id: String,
    pub key: String,
    pub version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretMetadata {
    pub id: String,
    pub key: String,
    pub version: u32,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectiveMcp {
    pub definition: McpDefinition,
    pub enabled: bool,
    pub secret: Option<SecretRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfigSnapshot {
    pub schema_version: u32,
    pub deployment_id: String,
    pub user: UserInfo,
    pub revision: u32,
    pub validated_at: i64,
    pub expires_at: i64,
    pub session_expires_at: i64,
    pub mcps: Vec<EffectiveMcp>,
}

#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsUpdate {
    pub expected_revision: u32,
    pub enabled: bool,
    pub secret_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretWrite {
    pub key: String,
    pub value: String,
    pub expected_version: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretRename {
    pub key: String,
    pub expected_version: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionInput {
    pub expected_version: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OAuthStatus {
    pub connected: bool,
    pub version: Option<u32>,
    pub expires_at: Option<i64>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CredentialRequest {
    pub expected_revision: u32,
    pub installation_id: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OAuthStoreRequest {
    pub expected_revision: u32,
    pub installation_id: String,
    pub expected_version: Option<u32>,
    pub refresh_lease: Option<String>,
    pub credentials: OAuthCredentials,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RefreshRequest {
    pub installation_id: String,
    pub expected_version: u32,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RefreshLease {
    pub lease_id: String,
    pub expires_at: i64,
    pub version: u32,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RefreshRelease {
    pub installation_id: String,
    pub lease_id: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OAuthDisconnect {
    pub expected_revision: u32,
    pub installation_id: String,
    pub expected_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InventoryReport {
    pub mcp_id: String,
    pub names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Unauthorized,
    Forbidden,
    InvalidRequest,
    Conflict,
    Offline,
    ConfigExpired,
    InvalidCache,
    AuthRequired,
    InstallFailed,
    UpstreamUnavailable,
    Unsupported,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    pub revision: Option<u32>,
    pub snapshot: Option<ConfigSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct UiContext {
    pub mode: String,
    pub user: Option<UserInfo>,
    pub server_url: Option<String>,
    pub online: bool,
    pub expires_at: Option<i64>,
    pub csrf_token: Option<String>,
}

/// Only endpoint credential routes may serialize these values. No Debug/TS export.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OAuthCredentials {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: i64,
    pub issuer: String,
    pub resource: String,
    pub client_id: String,
    pub scopes: Vec<String>,
}

/// Central login credentials are distinct from every upstream token.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EndpointSession {
    pub token: String,
    pub user: UserInfo,
    pub deployment_id: String,
    pub expires_at: i64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UpstreamCredential {
    Bearer {
        token: String,
        secret_id: String,
        version: u32,
    },
    #[serde(rename = "oauth")]
    OAuth {
        credentials: OAuthCredentials,
        version: u32,
        installation_id: String,
    },
}

/// Local browser actions are pinned to the displayed login generation, not just the deployment revision.
#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalAction<T> {
    pub expected_generation: String,
    pub input: T,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallRequest {
    pub expected_revision: u32,
    pub enable_after_install: bool,
    pub secret_id: Option<String>,
    pub rerun: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum InstallState {
    Running,
    Failed,
    Cancelled,
    Expired,
    Revoked,
    InstalledDisabled,
    Enabled,
    Interrupted,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallProgress {
    pub id: String,
    pub mcp_id: String,
    pub generation: String,
    pub state: InstallState,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub elapsed_seconds: u64,
    pub exit_code: Option<i32>,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalMcpMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub transport: String,
    pub auth: String,
    pub enabled: bool,
    pub secret: Option<SecretRef>,
    pub installation_available: bool,
    pub installed: bool,
    pub availability: String,
    pub auth_required: bool,
    pub tool_policy: ToolPolicy,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalCatalog {
    pub generation: String,
    pub revision: u32,
    pub expires_at: i64,
    pub mcps: Vec<LocalMcpMetadata>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretMutationOutcome {
    pub confirmed: bool,
    pub synchronized: bool,
    pub metadata: Option<SecretMetadata>,
}

/// Local errors never return native runtime commands or server-provided diagnostic text.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalApiError {
    pub code: ErrorCode,
    pub message: String,
    pub revision: Option<u32>,
    pub current: Option<LocalCatalog>,
}
