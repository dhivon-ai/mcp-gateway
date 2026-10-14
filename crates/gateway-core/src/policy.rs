use std::collections::HashSet;

use thiserror::Error;

use crate::{
    Assignment, AuthMode, CONFIG_SCHEMA_VERSION, CommandSpec, ConfigSnapshot, EffectiveMcp,
    GrantTarget, MAX_CONFIG_AGE_SECONDS, McpDefinition, ToolPolicy, Transport,
};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PolicyError {
    #[error("configuration belongs to another deployment or user")]
    Identity,
    #[error("configuration has expired; reconnect to the control plane")]
    Expired,
    #[error("configuration integrity or schema is invalid")]
    Invalid,
    #[error("system clock moved backwards; reconnect to the control plane")]
    ClockRollback,
    #[error("the current identity is disabled")]
    DisabledIdentity,
    #[error("this MCP is not assigned or is disabled")]
    Unassigned,
    #[error("this MCP has not been enabled")]
    NotEnabled,
    #[error("upstream authentication is required")]
    AuthRequired,
    #[error("this tool is blocked by administrator policy")]
    ToolDenied,
}

pub fn has_assignment(
    assignments: &[Assignment],
    user_id: &str,
    group_ids: &[String],
    mcp_id: &str,
) -> bool {
    assignments.iter().any(|a| {
        a.mcp_id == mcp_id
            && match &a.target {
                GrantTarget::All {} => true,
                GrantTarget::User { id } => id == user_id,
                GrantTarget::Group { id } => group_ids.contains(id),
            }
    })
}

impl ToolPolicy {
    pub fn allows(&self, original_tool_name: &str) -> bool {
        match self {
            Self::Allowlist(names) => names.iter().any(|n| n == original_tool_name),
            Self::Denylist(names) => !names.iter().any(|n| n == original_tool_name),
        }
    }
}

impl CommandSpec {
    pub fn validate(&self) -> Result<(), PolicyError> {
        if self.program.trim().is_empty()
            || self.program.len() > 4096
            || self.program.contains('\0')
            || self.args.len() > 128
            || self.env.len() > 128
            || self
                .args
                .iter()
                .any(|v| v.len() > 16_384 || v.contains('\0'))
            || self.env.iter().any(|(k, v)| {
                k.is_empty() || k.contains(['\0', '=']) || v.len() > 16_384 || v.contains('\0')
            })
            || self
                .cwd
                .as_ref()
                .is_some_and(|v| v.len() > 4096 || v.contains('\0'))
        {
            return Err(PolicyError::Invalid);
        }
        Ok(())
    }
}

impl McpDefinition {
    pub fn validate(&self) -> Result<(), PolicyError> {
        if self.id.is_empty()
            || self.id.len() > 128
            || self.name.trim().is_empty()
            || self.name.len() > 200
            || self.description.len() > 4096
        {
            return Err(PolicyError::Invalid);
        }
        match &self.transport {
            Transport::Stdio { command } => {
                command.validate()?;
                if !matches!(self.auth, AuthMode::None {}) {
                    return Err(PolicyError::Invalid);
                }
            }
            Transport::Http { url } => {
                if url.len() > 4096
                    || !(url.starts_with("https://") || url.starts_with("http://"))
                    || url.contains(['\0', '\n', '\r'])
                {
                    return Err(PolicyError::Invalid);
                }
                let parsed = url::Url::parse(url).map_err(|_| PolicyError::Invalid)?;
                if parsed.host_str().is_none()
                    || !parsed.username().is_empty()
                    || parsed.password().is_some()
                    || parsed.fragment().is_some()
                {
                    return Err(PolicyError::Invalid);
                }
            }
        }
        if let Some(command) = &self.install {
            command.validate()?;
        }
        if let AuthMode::OAuth { client_id, scopes } = &self.auth
            && (client_id.trim().is_empty()
                || client_id.len() > 512
                || scopes.len() > 64
                || scopes.iter().any(|s| s.is_empty() || s.len() > 256))
        {
            return Err(PolicyError::Invalid);
        }
        let names = match &self.tool_policy {
            ToolPolicy::Allowlist(v) | ToolPolicy::Denylist(v) => v,
        };
        if names.len() > 2048
            || names
                .iter()
                .any(|n| n.is_empty() || n.len() > 512 || n.contains('\0'))
        {
            return Err(PolicyError::Invalid);
        }
        Ok(())
    }
}

impl ConfigSnapshot {
    pub fn validate(
        &self,
        deployment_id: &str,
        user_id: &str,
        now: i64,
    ) -> Result<(), PolicyError> {
        if self.deployment_id != deployment_id || self.user.id != user_id {
            return Err(PolicyError::Identity);
        }
        if !self.user.active {
            return Err(PolicyError::DisabledIdentity);
        }
        let age_limit = self
            .expires_at
            .checked_sub(self.validated_at)
            .ok_or(PolicyError::Invalid)?;
        if self.schema_version != CONFIG_SCHEMA_VERSION
            || self.deployment_id.is_empty()
            || self.user.id.is_empty()
            || self.validated_at < 0
            || !(1..=MAX_CONFIG_AGE_SECONDS).contains(&age_limit)
            || self.expires_at > self.session_expires_at
            || self.session_expires_at > 253_402_300_799
            || self.mcps.len() > 512
        {
            return Err(PolicyError::Invalid);
        }
        if now < self.validated_at {
            return Err(PolicyError::ClockRollback);
        }
        if now >= self.expires_at {
            return Err(PolicyError::Expired);
        }
        let mut ids = HashSet::new();
        for mcp in &self.mcps {
            mcp.definition.validate()?;
            if !ids.insert(&mcp.definition.id) {
                return Err(PolicyError::Invalid);
            }
            if mcp.enabled
                && matches!(mcp.definition.auth, AuthMode::Bearer {})
                && mcp.secret.is_none()
            {
                return Err(PolicyError::Invalid);
            }
            if mcp
                .secret
                .as_ref()
                .is_some_and(|s| s.id.is_empty() || s.key.trim().is_empty() || s.version == 0)
            {
                return Err(PolicyError::Invalid);
            }
        }
        Ok(())
    }

    pub fn authorize_tool(
        &self,
        deployment_id: &str,
        user_id: &str,
        now: i64,
        mcp_id: &str,
        original_tool_name: &str,
        auth_ready: bool,
    ) -> Result<&EffectiveMcp, PolicyError> {
        self.validate(deployment_id, user_id, now)?;
        let mcp = self
            .mcps
            .iter()
            .find(|m| m.definition.id == mcp_id)
            .ok_or(PolicyError::Unassigned)?;
        if !mcp.definition.active {
            return Err(PolicyError::Unassigned);
        }
        if !mcp.enabled {
            return Err(PolicyError::NotEnabled);
        }
        if !auth_ready {
            return Err(PolicyError::AuthRequired);
        }
        if !mcp.definition.tool_policy.allows(original_tool_name) {
            return Err(PolicyError::ToolDenied);
        }
        Ok(mcp)
    }
}

/// HTTP is permitted only for explicitly enabled, isolated loopback fixtures.
pub fn validate_remote_url(
    raw: &str,
    allow_insecure_loopback: bool,
) -> Result<url::Url, PolicyError> {
    let parsed = url::Url::parse(raw).map_err(|_| PolicyError::Invalid)?;
    let loopback = parsed
        .host_str()
        .is_some_and(|h| h == "localhost" || h == "127.0.0.1" || h == "[::1]");
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.fragment().is_some()
        || parsed.host_str().is_none()
        || !(parsed.scheme() == "https"
            || (allow_insecure_loopback && loopback && parsed.scheme() == "http"))
    {
        return Err(PolicyError::Invalid);
    }
    Ok(parsed)
}
