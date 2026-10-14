use crate::{
    AppError, AppState, Input, bump_revision, epoch,
    identity::{self, Principal},
    uuid,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use gateway_core::{
    AdminConfiguration, Assignment, AssignmentWrite, AuthMode, CONFIG_SCHEMA_VERSION, CatalogWrite,
    ConfigSnapshot, EffectiveMcp, GrantTarget, InventoryReport, MAX_CONFIG_AGE_SECONDS,
    McpDefinition, RevisionInfo, RevisionInput, SecretRef, SettingsUpdate, ToolInventory,
    Transport,
};
use sqlx::{Postgres, Transaction, types::Json as DbJson};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct SnapshotRow {
    definition: DbJson<McpDefinition>,
    enabled_user: Option<Uuid>,
    secret_id: Option<Uuid>,
    key: Option<String>,
    version: Option<i64>,
}
#[derive(sqlx::FromRow)]
struct AssignmentRow {
    id: Uuid,
    mcp_id: Uuid,
    target_kind: String,
    target_user: Option<Uuid>,
    target_group: Option<Uuid>,
}

pub(crate) fn revision(value: i64) -> Result<u32, AppError> {
    u32::try_from(value).map_err(|_| AppError::internal())
}
pub(crate) fn check_revision(expected: u32, current: u32) -> Result<(), AppError> {
    if expected != current {
        let mut e =
            AppError::conflict("Configuration changed. Review the current state before retrying.");
        e.revision = Some(current);
        return Err(e);
    }
    Ok(())
}
pub(crate) async fn current_revision(tx: &mut Transaction<'_, Postgres>) -> Result<u32, AppError> {
    revision(
        sqlx::query_scalar("SELECT revision FROM deployment WHERE singleton=true")
            .fetch_one(&mut **tx)
            .await?,
    )
}
fn validate_definition(state: &AppState, definition: &McpDefinition) -> Result<Uuid, AppError> {
    let id = uuid(&definition.id)?;
    definition
        .validate()
        .map_err(|_| AppError::bad("Invalid MCP definition."))?;
    if let Transport::Http { url } = &definition.transport {
        gateway_core::validate_remote_url(url, state.options.allow_loopback_http)
            .map_err(|_| AppError::bad("Remote MCP URL must use verified HTTPS."))?;
    }
    Ok(id)
}
async fn catalog_budget(tx: &mut Transaction<'_, Postgres>) -> Result<(), AppError> {
    let bytes: i64 = sqlx::query_scalar(
        "SELECT COALESCE(sum(octet_length(definition::text)),0)::bigint FROM mcps",
    )
    .fetch_one(&mut **tx)
    .await?;
    if bytes > 1024 * 1024 {
        return Err(AppError::bad(
            "Catalog definitions exceed the 1 MiB endpoint budget. Reduce command or tool metadata.",
        ));
    }
    Ok(())
}
/// Delete opt-ins whenever their final grant disappears. Regranting is a new opt-in.
pub(crate) async fn reconcile(tx: &mut Transaction<'_, Postgres>) -> Result<(), AppError> {
    sqlx::query("DELETE FROM user_settings s WHERE NOT EXISTS (SELECT 1 FROM mcps m JOIN users u ON u.id=s.user_id WHERE m.id=s.mcp_id AND u.active AND m.definition->>'active'='true' AND EXISTS (SELECT 1 FROM assignments a WHERE a.mcp_id=m.id AND (a.target_kind='all' OR a.target_user=s.user_id OR EXISTS (SELECT 1 FROM memberships g WHERE g.user_id=s.user_id AND g.group_id=a.target_group))))").execute(&mut **tx).await?;
    sqlx::query("DELETE FROM tool_inventory i WHERE NOT EXISTS (SELECT 1 FROM assignments a WHERE a.mcp_id=i.mcp_id AND (a.target_kind='all' OR a.target_user=i.user_id OR EXISTS(SELECT 1 FROM memberships g WHERE g.user_id=i.user_id AND g.group_id=a.target_group)))").execute(&mut **tx).await?;
    sqlx::query("DELETE FROM oauth_credentials c WHERE NOT EXISTS(SELECT 1 FROM mcps m JOIN users u ON u.id=c.user_id WHERE m.id=c.mcp_id AND u.active AND m.definition->>'active'='true' AND EXISTS(SELECT 1 FROM assignments a WHERE a.mcp_id=m.id AND (a.target_kind='all' OR a.target_user=c.user_id OR EXISTS(SELECT 1 FROM memberships g WHERE g.user_id=c.user_id AND g.group_id=a.target_group))))").execute(&mut **tx).await?;
    Ok(())
}
pub(crate) async fn assigned(
    tx: &mut Transaction<'_, Postgres>,
    user: Uuid,
    mcp: Uuid,
) -> Result<bool, AppError> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM mcps m WHERE m.id=$2 AND m.definition->>'active'='true' AND EXISTS(SELECT 1 FROM assignments a WHERE a.mcp_id=m.id AND (a.target_kind='all' OR a.target_user=$1 OR EXISTS(SELECT 1 FROM memberships g WHERE g.user_id=$1 AND g.group_id=a.target_group))))").bind(user).bind(mcp).fetch_one(&mut **tx).await?)
}
pub(crate) async fn definition(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<McpDefinition, AppError> {
    let row: Option<DbJson<McpDefinition>> =
        sqlx::query_scalar("SELECT definition FROM mcps WHERE id=$1")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?;
    row.map(|j| j.0)
        .ok_or_else(|| AppError::bad("Unknown MCP."))
}
pub(crate) async fn snapshot(
    state: &AppState,
    tx: &mut Transaction<'_, Postgres>,
    p: &Principal,
) -> Result<ConfigSnapshot, AppError> {
    let user = uuid(&p.user.id)?;
    let rows:Vec<SnapshotRow>=sqlx::query_as("SELECT m.definition,s.user_id AS enabled_user,k.id AS secret_id,k.key,k.version FROM mcps m LEFT JOIN user_settings s ON s.mcp_id=m.id AND s.user_id=$1 LEFT JOIN user_secrets k ON k.id=s.secret_id AND k.owner_id=$1 WHERE m.definition->>'active'='true' AND EXISTS(SELECT 1 FROM assignments a WHERE a.mcp_id=m.id AND (a.target_kind='all' OR a.target_user=$1 OR EXISTS(SELECT 1 FROM memberships g WHERE g.user_id=$1 AND g.group_id=a.target_group))) ORDER BY m.id").bind(user).fetch_all(&mut **tx).await?;
    let mut mcps = Vec::new();
    for SnapshotRow {
        definition: def,
        enabled_user: enabled,
        secret_id,
        key,
        version,
    } in rows
    {
        let secret = match (secret_id, key, version) {
            (Some(id), Some(key), Some(v)) => Some(SecretRef {
                id: id.to_string(),
                key,
                version: revision(v)?,
            }),
            (None, None, None) => None,
            _ => return Err(AppError::internal()),
        };
        mcps.push(EffectiveMcp {
            definition: def.0,
            enabled: enabled.is_some(),
            secret,
        });
    }
    let now = epoch()?;
    let result = ConfigSnapshot {
        schema_version: CONFIG_SCHEMA_VERSION,
        deployment_id: state.deployment_id.to_string(),
        user: p.user.clone(),
        revision: current_revision(tx).await?,
        validated_at: now,
        expires_at: (now + MAX_CONFIG_AGE_SECONDS).min(p.expires_at),
        session_expires_at: p.expires_at,
        mcps,
    };
    result
        .validate(&result.deployment_id, &p.user.id, now)
        .map_err(|_| AppError::internal())?;
    Ok(result)
}
pub async fn get_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ConfigSnapshot>, AppError> {
    let p = identity::authenticate(&state, &headers, false).await?;
    let (mut tx, p, _) = identity::begin_user(&state, &p).await?;
    let result = snapshot(&state, &mut tx, &p).await?;
    tx.commit().await?;
    Ok(Json(result))
}
pub async fn admin_configuration(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminConfiguration>, AppError> {
    let p = identity::admin(&state, &headers, false).await?;
    let mut tx = identity::begin_admin(&state, &p).await?;
    let mcps: Vec<DbJson<McpDefinition>> =
        sqlx::query_scalar("SELECT definition FROM mcps ORDER BY id")
            .fetch_all(&mut *tx)
            .await?;
    let rows: Vec<AssignmentRow> = sqlx::query_as(
        "SELECT id,mcp_id,target_kind,target_user,target_group FROM assignments ORDER BY id",
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut assignments = Vec::new();
    for AssignmentRow {
        id,
        mcp_id,
        target_kind: kind,
        target_user: user,
        target_group: group,
    } in rows
    {
        let target = match (kind.as_str(), user, group) {
            ("all", None, None) => GrantTarget::All {},
            ("user", Some(id), None) => GrantTarget::User { id: id.to_string() },
            ("group", None, Some(id)) => GrantTarget::Group { id: id.to_string() },
            _ => return Err(AppError::internal()),
        };
        assignments.push(Assignment {
            id: id.to_string(),
            mcp_id: mcp_id.to_string(),
            target,
        });
    }
    // One bounded latest observation per MCP prevents clients expanding inventory
    // through many reports. Metadata never alters authorization.
    let observations:Vec<(Uuid,Vec<String>,i64)>=sqlx::query_as("SELECT DISTINCT ON(mcp_id) mcp_id,names,observed_at FROM tool_inventory ORDER BY mcp_id,observed_at DESC").fetch_all(&mut *tx).await?;
    let inventory = observations
        .into_iter()
        .map(|(id, names, observed_at)| ToolInventory {
            mcp_id: id.to_string(),
            names,
            observed_at,
        })
        .collect();
    let result = AdminConfiguration {
        revision: current_revision(&mut tx).await?,
        mcps: mcps.into_iter().map(|j| j.0).collect(),
        assignments,
        inventory,
    };
    tx.commit().await?;
    Ok(Json(result))
}
pub async fn create_mcp(
    State(state): State<AppState>,
    headers: HeaderMap,
    Input(input): Input<CatalogWrite>,
) -> Result<Json<RevisionInfo>, AppError> {
    let p = identity::admin(&state, &headers, true).await?;
    let id = validate_definition(&state, &input.definition)?;
    let mut tx = identity::begin_admin(&state, &p).await?;
    check_revision(input.expected_revision, current_revision(&mut tx).await?)?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM mcps")
        .fetch_one(&mut *tx)
        .await?;
    if count >= 512 {
        return Err(AppError::bad("Catalog limit is 512 MCPs."));
    }
    sqlx::query("INSERT INTO mcps(id,definition) VALUES($1,$2)")
        .bind(id)
        .bind(DbJson(input.definition))
        .execute(&mut *tx)
        .await?;
    catalog_budget(&mut tx).await?;
    bump_revision(&mut tx).await?;
    let revision = current_revision(&mut tx).await?;
    tx.commit().await?;
    tracing::info!(event="mcp_created",actor=%p.user.id,mcp_id=%id);
    Ok(Json(RevisionInfo { revision }))
}
pub async fn edit_mcp(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(raw): Path<String>,
    Input(input): Input<CatalogWrite>,
) -> Result<Json<RevisionInfo>, AppError> {
    let p = identity::admin(&state, &headers, true).await?;
    let id = validate_definition(&state, &input.definition)?;
    if uuid(&raw)? != id {
        return Err(AppError::bad("MCP identifiers must match."));
    }
    let mut tx = identity::begin_admin(&state, &p).await?;
    check_revision(input.expected_revision, current_revision(&mut tx).await?)?;
    let old = definition(&mut tx, id).await?;
    if old.transport != input.definition.transport || old.auth != input.definition.auth {
        sqlx::query("DELETE FROM oauth_credentials WHERE mcp_id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM user_settings WHERE mcp_id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM tool_inventory WHERE mcp_id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("UPDATE mcps SET definition=$2 WHERE id=$1")
        .bind(id)
        .bind(DbJson(input.definition))
        .execute(&mut *tx)
        .await?;
    catalog_budget(&mut tx).await?;
    reconcile(&mut tx).await?;
    bump_revision(&mut tx).await?;
    let revision = current_revision(&mut tx).await?;
    tx.commit().await?;
    tracing::info!(event="mcp_updated",actor=%p.user.id,mcp_id=%id);
    Ok(Json(RevisionInfo { revision }))
}
pub async fn delete_mcp(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(raw): Path<String>,
    Input(input): Input<RevisionInput>,
) -> Result<Json<RevisionInfo>, AppError> {
    let p = identity::admin(&state, &headers, true).await?;
    let id = uuid(&raw)?;
    let mut tx = identity::begin_admin(&state, &p).await?;
    check_revision(input.expected_revision, current_revision(&mut tx).await?)?;
    let result = sqlx::query("DELETE FROM mcps WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() != 1 {
        return Err(AppError::bad("Unknown MCP."));
    }
    bump_revision(&mut tx).await?;
    let revision = current_revision(&mut tx).await?;
    tx.commit().await?;
    tracing::info!(event="mcp_deleted",actor=%p.user.id,mcp_id=%id);
    Ok(Json(RevisionInfo { revision }))
}
pub async fn create_assignment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Input(input): Input<AssignmentWrite>,
) -> Result<Json<Assignment>, AppError> {
    let p = identity::admin(&state, &headers, true).await?;
    let mcp = uuid(&input.mcp_id)?;
    let mut tx = identity::begin_admin(&state, &p).await?;
    check_revision(input.expected_revision, current_revision(&mut tx).await?)?;
    definition(&mut tx, mcp).await?;
    let (kind, user, group) = match &input.target {
        GrantTarget::All {} => ("all", None, None),
        GrantTarget::User { id } => {
            let id = uuid(id)?;
            let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id=$1)")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
            if !exists {
                return Err(AppError::bad("Unknown user."));
            }
            ("user", Some(id), None)
        }
        GrantTarget::Group { id } => {
            let id = uuid(id)?;
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM groups WHERE id=$1)")
                    .bind(id)
                    .fetch_one(&mut *tx)
                    .await?;
            if !exists {
                return Err(AppError::bad("Unknown group."));
            }
            ("group", None, Some(id))
        }
    };
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO assignments(id,mcp_id,target_kind,target_user,target_group) VALUES($1,$2,$3,$4,$5)").bind(id).bind(mcp).bind(kind).bind(user).bind(group).execute(&mut *tx).await?;
    bump_revision(&mut tx).await?;
    tx.commit().await?;
    tracing::info!(event="assignment_created",actor=%p.user.id,mcp_id=%mcp,assignment_id=%id);
    Ok(Json(Assignment {
        id: id.to_string(),
        mcp_id: mcp.to_string(),
        target: input.target,
    }))
}
pub async fn delete_assignment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(raw): Path<String>,
    Input(input): Input<RevisionInput>,
) -> Result<Json<RevisionInfo>, AppError> {
    let p = identity::admin(&state, &headers, true).await?;
    let id = uuid(&raw)?;
    let mut tx = identity::begin_admin(&state, &p).await?;
    check_revision(input.expected_revision, current_revision(&mut tx).await?)?;
    let result = sqlx::query("DELETE FROM assignments WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() != 1 {
        return Err(AppError::bad("Unknown assignment."));
    }
    reconcile(&mut tx).await?;
    bump_revision(&mut tx).await?;
    let revision = current_revision(&mut tx).await?;
    tx.commit().await?;
    tracing::info!(event="assignment_deleted",actor=%p.user.id,assignment_id=%id);
    Ok(Json(RevisionInfo { revision }))
}
pub async fn update_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(raw): Path<String>,
    Input(input): Input<SettingsUpdate>,
) -> Result<Json<ConfigSnapshot>, AppError> {
    let p = identity::authenticate(&state, &headers, true).await?;
    let mcp = uuid(&raw)?;
    let (mut tx, p, current) = identity::begin_user(&state, &p).await?;
    let user = uuid(&p.user.id)?;
    if let Err(mut error) = check_revision(input.expected_revision, current) {
        error.snapshot = Some(Box::new(snapshot(&state, &mut tx, &p).await?));
        return Err(error);
    }
    if !assigned(&mut tx, user, mcp).await? {
        return Err(AppError::forbidden());
    }
    let def = definition(&mut tx, mcp).await?;
    if input.enabled {
        let secret = match (&def.auth, input.secret_id) {
            (AuthMode::Bearer {}, Some(id)) => {
                let id = uuid(&id)?;
                let own: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM user_secrets WHERE id=$1 AND owner_id=$2)",
                )
                .bind(id)
                .bind(user)
                .fetch_one(&mut *tx)
                .await?;
                if !own {
                    return Err(AppError::forbidden());
                }
                Some(id)
            }
            (AuthMode::Bearer {}, None) => {
                return Err(AppError::bad("Select an owned bearer secret key."));
            }
            (_, None) => None,
            (_, Some(_)) => return Err(AppError::bad("This MCP does not use a bearer secret.")),
        };
        sqlx::query("INSERT INTO user_settings(user_id,mcp_id,secret_id) VALUES($1,$2,$3) ON CONFLICT(user_id,mcp_id) DO UPDATE SET secret_id=excluded.secret_id").bind(user).bind(mcp).bind(secret).execute(&mut *tx).await?;
    } else {
        if input.secret_id.is_some() {
            return Err(AppError::bad(
                "Disabled MCPs cannot retain a secret binding.",
            ));
        }
        sqlx::query("DELETE FROM user_settings WHERE user_id=$1 AND mcp_id=$2")
            .bind(user)
            .bind(mcp)
            .execute(&mut *tx)
            .await?;
    }
    bump_revision(&mut tx).await?;
    let result = snapshot(&state, &mut tx, &p).await?;
    tx.commit().await?;
    tracing::info!(event="settings_confirmed",user_id=%p.user.id,mcp_id=%mcp,enabled=input.enabled,revision=result.revision);
    Ok(Json(result))
}
pub async fn report_inventory(
    State(state): State<AppState>,
    headers: HeaderMap,
    Input(mut input): Input<InventoryReport>,
) -> Result<StatusCode, AppError> {
    let p = identity::authenticate(&state, &headers, true).await?;
    if !p.endpoint {
        return Err(AppError::forbidden());
    }
    if input.names.len() > 2048
        || input
            .names
            .iter()
            .any(|n| n.is_empty() || n.len() > 512 || n.contains(['\0', '\r', '\n']))
    {
        return Err(AppError::bad("Invalid tool inventory."));
    }
    input.names.sort();
    input.names.dedup();
    let mcp = uuid(&input.mcp_id)?;
    let (mut tx, p, _) = identity::begin_user(&state, &p).await?;
    let user = uuid(&p.user.id)?;
    if !assigned(&mut tx, user, mcp).await? {
        return Err(AppError::forbidden());
    }
    sqlx::query("INSERT INTO tool_inventory(mcp_id,user_id,names,observed_at) VALUES($1,$2,$3,$4) ON CONFLICT(mcp_id,user_id) DO UPDATE SET names=excluded.names,observed_at=excluded.observed_at").bind(mcp).bind(user).bind(input.names).bind(epoch()?).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
