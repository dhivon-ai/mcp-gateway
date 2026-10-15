use std::{collections::HashSet, net::SocketAddr};

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier},
};
use axum::{
    Extension, Json,
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use gateway_core::{
    EndpointSession, ErrorCode, GroupInfo, GroupWrite, LoginRequest, PasswordReset, Role,
    UiContext, UserCreate, UserEdit, UserInfo,
};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

use crate::{AppError, AppState, Input, bump_revision, empty_context, epoch, lock_revision, uuid};

const SESSION_SECONDS: i64 = 12 * 3600;
#[derive(FromRow)]
struct UserRow {
    id: Uuid,
    email: String,
    name: String,
    role: String,
    active: bool,
}
impl UserRow {
    fn info(self) -> UserInfo {
        UserInfo {
            id: self.id.to_string(),
            email: self.email,
            name: self.name,
            role: if self.role == "admin" {
                Role::Admin
            } else {
                Role::User
            },
            active: self.active,
        }
    }
}
#[derive(FromRow)]
struct SessionRow {
    id: Uuid,
    email: String,
    name: String,
    role: String,
    active: bool,
    csrf_token: String,
    expires_at: i64,
}
pub(crate) struct Principal {
    pub user: UserInfo,
    pub csrf: String,
    pub token_hash: Vec<u8>,
    pub endpoint: bool,
    pub expires_at: i64,
}

pub fn email(raw: &str) -> Result<String, AppError> {
    let value = raw.trim().to_ascii_lowercase();
    let invalid = || AppError::bad("Enter a valid email address (at most 254 ASCII bytes).");
    let (local, domain) = value.split_once('@').ok_or_else(invalid)?;
    if !value.is_ascii()
        || value.len() > 254
        || local.is_empty()
        || local.len() > 64
        || local.starts_with('.')
        || local.ends_with('.')
        || local.contains("..")
        || !local
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".!#$%&'*+-/=?^_`{|}~".contains(&b))
        || domain.is_empty()
        || domain.len() > 253
        || domain.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
    {
        return Err(invalid());
    }
    Ok(value)
}
pub fn display_name(raw: &str) -> Result<String, AppError> {
    let value = raw.trim();
    if value.is_empty() || value.len() > 200 || value.chars().any(char::is_control) {
        return Err(AppError::bad(
            "Name must contain 1–200 UTF-8 bytes without control characters.",
        ));
    }
    Ok(value.to_owned())
}
fn check_password(password: &str) -> Result<(), AppError> {
    if !(12..=128).contains(&password.len()) {
        Err(AppError::bad("Password must contain 12–128 bytes."))
    } else {
        Ok(())
    }
}
pub async fn hash_password(password: String) -> Result<String, AppError> {
    tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes())
            .map(|p| p.to_string())
            .map_err(|_| AppError::internal())
    })
    .await
    .map_err(|_| AppError::internal())?
}
fn opaque() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}
fn valid_token(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn token_hash(value: &str) -> Vec<u8> {
    Sha256::digest(value.as_bytes()).to_vec()
}
fn cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|p| {
            let (k, v) = p.trim().split_once('=')?;
            (k == name && valid_token(v)).then(|| v.to_owned())
        })
}
fn set_cookie(
    response: &mut Response,
    state: &AppState,
    name: &str,
    value: &str,
    age: i64,
) -> Result<(), AppError> {
    let secure = if state.secure_cookies { "; Secure" } else { "" };
    let value = HeaderValue::from_str(&format!(
        "{name}={value}; Path=/; HttpOnly; SameSite=Strict; Max-Age={age}{secure}"
    ))
    .map_err(|_| AppError::internal())?;
    response.headers_mut().append(header::SET_COOKIE, value);
    Ok(())
}
fn credentials(headers: &HeaderMap) -> Result<(Vec<u8>, bool), AppError> {
    if let Some(auth) = headers.get(header::AUTHORIZATION) {
        let token = auth
            .to_str()
            .ok()
            .and_then(|s| s.strip_prefix("Bearer "))
            .filter(|s| valid_token(s))
            .ok_or_else(AppError::unauthorized)?;
        Ok((token_hash(token), true))
    } else {
        Ok((
            token_hash(&cookie(headers, "gateway_session").ok_or_else(AppError::unauthorized)?),
            false,
        ))
    }
}
fn principal(row: SessionRow, hash: Vec<u8>, endpoint: bool) -> Principal {
    Principal {
        user: UserRow {
            id: row.id,
            email: row.email,
            name: row.name,
            role: row.role,
            active: row.active,
        }
        .info(),
        csrf: row.csrf_token,
        expires_at: row.expires_at,
        token_hash: hash,
        endpoint,
    }
}
const SESSION_QUERY: &str = "SELECT u.id,u.email,u.name,u.role,u.active,s.csrf_token,s.expires_at FROM sessions s JOIN users u ON u.id=s.user_id WHERE s.token_hash=$1 AND s.expires_at>$2 AND u.active";
pub(crate) async fn authenticate(
    state: &AppState,
    headers: &HeaderMap,
    mutation: bool,
) -> Result<Principal, AppError> {
    let (hash, endpoint) = credentials(headers)?;
    let row = sqlx::query_as::<_, SessionRow>(SESSION_QUERY)
        .bind(&hash)
        .bind(epoch()?)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(AppError::unauthorized)?;
    let p = principal(row, hash, endpoint);
    if mutation
        && !p.endpoint
        && headers.get("x-csrf-token").and_then(|s| s.to_str().ok()) != Some(p.csrf.as_str())
    {
        return Err(AppError::forbidden());
    }
    Ok(p)
}
pub(crate) async fn admin(
    state: &AppState,
    headers: &HeaderMap,
    mutation: bool,
) -> Result<Principal, AppError> {
    let p = authenticate(state, headers, mutation).await?;
    if p.user.role != Role::Admin {
        return Err(AppError::forbidden());
    }
    Ok(p)
}
/// Revalidate under the same serialized authority lock as account and policy writes.
pub(crate) async fn begin_admin<'a>(
    state: &'a AppState,
    p: &Principal,
) -> Result<Transaction<'a, Postgres>, AppError> {
    let mut tx = state.pool.begin().await?;
    lock_revision(&mut tx).await?;
    let row = sqlx::query_as::<_, SessionRow>(SESSION_QUERY)
        .bind(&p.token_hash)
        .bind(epoch()?)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(AppError::unauthorized)?;
    if row.role != "admin" {
        return Err(AppError::forbidden());
    }
    Ok(tx)
}

pub(crate) async fn begin_user<'a>(
    state: &'a AppState,
    p: &Principal,
) -> Result<(Transaction<'a, Postgres>, Principal, u32), AppError> {
    let mut tx = state.pool.begin().await?;
    let revision = crate::configuration::revision(lock_revision(&mut tx).await?)?;
    let row = sqlx::query_as::<_, SessionRow>(SESSION_QUERY)
        .bind(&p.token_hash)
        .bind(epoch()?)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(AppError::unauthorized)?;
    let p = principal(row, p.token_hash.clone(), p.endpoint);
    Ok((tx, p, revision))
}

async fn begin_admin_checked<'a>(
    state: &'a AppState,
    p: &Principal,
    expected: u32,
) -> Result<Transaction<'a, Postgres>, AppError> {
    let mut tx = begin_admin(state, p).await?;
    let revision = crate::configuration::current_revision(&mut tx).await?;
    crate::configuration::check_revision(expected, revision)?;
    Ok(tx)
}
fn ui_context(state: &AppState, p: Principal) -> UiContext {
    let mut ctx = empty_context(state);
    ctx.user = Some(p.user);
    ctx.expires_at = Some(p.expires_at);
    ctx.csrf_token = Some(p.csrf);
    ctx
}
pub async fn context(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    if headers.contains_key(header::AUTHORIZATION) {
        return Ok(Json(ui_context(
            &state,
            authenticate(&state, &headers, false).await?,
        ))
        .into_response());
    }
    let stale_cookie = cookie(&headers, "gateway_session").is_some();
    if stale_cookie {
        match authenticate(&state, &headers, false).await {
            Ok(principal) => return Ok(Json(ui_context(&state, principal)).into_response()),
            Err(error) if error.status == StatusCode::UNAUTHORIZED => {}
            Err(error) => return Err(error),
        }
    }
    let token = cookie(&headers, "gateway_preauth").unwrap_or_else(opaque);
    let mut ctx = empty_context(&state);
    ctx.csrf_token = Some(token.clone());
    let mut response = Json(ctx).into_response();
    set_cookie(&mut response, &state, "gateway_preauth", &token, 600)?;
    if stale_cookie {
        set_cookie(&mut response, &state, "gateway_session", "", 0)?;
    }
    Ok(response)
}
async fn throttle(
    state: &AppState,
    name: &str,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
) -> Result<(), AppError> {
    let now = epoch()?;
    let mut attempts = state.attempts.lock().await;
    attempts.retain(|_, (time, _)| now >= *time && now - *time < 60);
    if attempts.len() >= 10000 {
        return Err(AppError::new(
            StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::Forbidden,
            "Login limit reached. Try again later.",
        ));
    }
    let address = peer
        .map(|p| p.0.0.ip().to_string())
        .unwrap_or_else(|| "local-test".into());
    for (key, max) in [(format!("u:{name}"), 5), (format!("ip:{address}"), 100)] {
        let item = attempts.entry(key).or_insert((now, 0));
        if item.1 >= max {
            return Err(AppError::new(
                StatusCode::TOO_MANY_REQUESTS,
                ErrorCode::Forbidden,
                "Login limit reached. Try again in a minute.",
            ));
        }
        item.1 += 1;
    }
    Ok(())
}
async fn login(
    state: &AppState,
    input: LoginRequest,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
) -> Result<EndpointSession, AppError> {
    let name = email(&input.email).map_err(|_| AppError::unauthorized())?;
    if input.password.len() > 128 {
        return Err(AppError::unauthorized());
    }
    throttle(state, &name, peer).await?;
    let found: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id,password_hash FROM users WHERE email=$1 AND active")
            .bind(&name)
            .fetch_optional(&state.pool)
            .await?;
    let hash = found
        .as_ref()
        .map(|(_, h)| h.clone())
        .unwrap_or_else(|| state.dummy_hash.clone());
    let verify_hash = hash.clone();
    let permit = state
        .password_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| {
            AppError::new(
                StatusCode::TOO_MANY_REQUESTS,
                ErrorCode::Forbidden,
                "Login is busy. Try again shortly.",
            )
        })?;
    let valid = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        Argon2::default()
            .verify_password(input.password.as_bytes(), verify_hash.as_str())
            .is_ok()
    })
    .await
    .map_err(|_| AppError::internal())?;
    let (id, _) = found.filter(|_| valid).ok_or_else(AppError::unauthorized)?;
    let now = epoch()?;
    sqlx::query("DELETE FROM sessions WHERE expires_at<=$1")
        .bind(now)
        .execute(&state.pool)
        .await?;
    let mut tx = state.pool.begin().await?;
    // Reset/disable/email-edit races cannot mint a session from superseded credentials.
    let row=sqlx::query_as::<_,UserRow>("SELECT id,email,name,role,active FROM users WHERE id=$1 AND active AND password_hash=$2 AND email=$3 FOR UPDATE").bind(id).bind(hash).bind(&name).fetch_optional(&mut *tx).await?.ok_or_else(AppError::unauthorized)?;
    let token = opaque();
    let csrf = opaque();
    let expires_at = now + SESSION_SECONDS;
    sqlx::query(
        "INSERT INTO sessions(token_hash,user_id,csrf_token,expires_at) VALUES($1,$2,$3,$4)",
    )
    .bind(token_hash(&token))
    .bind(id)
    .bind(csrf)
    .bind(expires_at)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    state.attempts.lock().await.remove(&format!("u:{name}"));
    tracing::info!(event="session_created",user_id=%id);
    Ok(EndpointSession {
        token,
        user: row.info(),
        deployment_id: state.deployment_id.to_string(),
        expires_at,
    })
}
pub async fn endpoint_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    Input(input): Input<LoginRequest>,
) -> Result<Json<EndpointSession>, AppError> {
    if headers.contains_key(header::COOKIE) {
        return Err(AppError::forbidden());
    }
    Ok(Json(login(&state, input, peer).await?))
}
pub async fn browser_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    Input(input): Input<LoginRequest>,
) -> Result<Response, AppError> {
    let preauth = cookie(&headers, "gateway_preauth").ok_or_else(AppError::forbidden)?;
    if headers.get("x-csrf-token").and_then(|s| s.to_str().ok()) != Some(preauth.as_str()) {
        return Err(AppError::forbidden());
    }
    let session = login(&state, input, peer).await?;
    let mut auth = HeaderMap::new();
    auth.insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", session.token))
            .map_err(|_| AppError::internal())?,
    );
    let mut response = Json(ui_context(
        &state,
        authenticate(&state, &auth, false).await?,
    ))
    .into_response();
    set_cookie(
        &mut response,
        &state,
        "gateway_session",
        &session.token,
        SESSION_SECONDS,
    )?;
    set_cookie(&mut response, &state, "gateway_preauth", "", 0)?;
    Ok(response)
}
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let p = authenticate(&state, &headers, true).await?;
    sqlx::query("DELETE FROM sessions WHERE token_hash=$1")
        .bind(p.token_hash)
        .execute(&state.pool)
        .await?;
    tracing::info!(event="session_revoked",user_id=%p.user.id);
    let mut response = StatusCode::NO_CONTENT.into_response();
    set_cookie(&mut response, &state, "gateway_session", "", 0)?;
    Ok(response)
}
fn email_write_error(error: sqlx::Error) -> AppError {
    if error
        .as_database_error()
        .is_some_and(|e| e.is_unique_violation())
    {
        AppError::bad("That email address is already in use.")
    } else {
        error.into()
    }
}
pub async fn bootstrap(
    state: &AppState,
    raw_email: &str,
    raw_name: &str,
    password: String,
) -> Result<UserInfo, AppError> {
    let email = email(raw_email)?;
    let name = display_name(raw_name)?;
    check_password(&password)?;
    let hash = hash_password(password).await?;
    let mut tx = state.pool.begin().await?;
    lock_revision(&mut tx).await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(&mut *tx)
        .await?;
    if count != 0 {
        return Err(AppError::conflict(
            "Bootstrap is only available for an empty deployment.",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO users(id,email,password_hash,role,name) VALUES($1,$2,$3,'admin',$4)")
        .bind(id)
        .bind(&email)
        .bind(hash)
        .bind(&name)
        .execute(&mut *tx)
        .await
        .map_err(email_write_error)?;
    bump_revision(&mut tx).await?;
    tx.commit().await?;
    Ok(UserInfo {
        id: id.to_string(),
        email,
        name,
        role: Role::Admin,
        active: true,
    })
}
pub async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<UserInfo>>, AppError> {
    admin(&state, &headers, false).await?;
    let rows =
        sqlx::query_as::<_, UserRow>("SELECT id,email,name,role,active FROM users ORDER BY email")
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(rows.into_iter().map(UserRow::info).collect()))
}
fn role(value: &Role) -> &'static str {
    match value {
        Role::Admin => "admin",
        Role::User => "user",
    }
}
pub async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Input(input): Input<UserCreate>,
) -> Result<Json<UserInfo>, AppError> {
    let p = admin(&state, &headers, true).await?;
    let email = email(&input.email)?;
    let name = display_name(&input.name)?;
    check_password(&input.password)?;
    let hash = hash_password(input.password).await?;
    let id = Uuid::new_v4();
    let mut tx = begin_admin_checked(&state, &p, input.expected_revision).await?;
    sqlx::query("INSERT INTO users(id,email,password_hash,role,name) VALUES($1,$2,$3,$4,$5)")
        .bind(id)
        .bind(&email)
        .bind(hash)
        .bind(role(&input.role))
        .bind(&name)
        .execute(&mut *tx)
        .await
        .map_err(email_write_error)?;
    bump_revision(&mut tx).await?;
    tx.commit().await?;
    tracing::info!(event="user_created",actor=%p.user.id,user_id=%id);
    Ok(Json(UserInfo {
        id: id.to_string(),
        email,
        name,
        role: input.role,
        active: true,
    }))
}
pub async fn edit_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Input(input): Input<UserEdit>,
) -> Result<Json<UserInfo>, AppError> {
    let p = admin(&state, &headers, true).await?;
    let id = uuid(&id)?;
    let email = email(&input.email)?;
    let name = display_name(&input.name)?;
    let mut tx = begin_admin_checked(&state, &p, input.expected_revision).await?;
    let current = sqlx::query_as::<_, UserRow>(
        "SELECT id,email,name,role,active FROM users WHERE id=$1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::bad("Unknown user."))?;
    if current.role == "admin" && current.active && (!input.active || input.role != Role::Admin) {
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM users WHERE active AND role='admin'")
                .fetch_one(&mut *tx)
                .await?;
        if count <= 1 {
            return Err(AppError::conflict(
                "The last active administrator cannot be removed.",
            ));
        }
    }
    sqlx::query("UPDATE users SET role=$2,active=$3,email=$4,name=$5 WHERE id=$1")
        .bind(id)
        .bind(role(&input.role))
        .bind(input.active)
        .bind(&email)
        .bind(&name)
        .execute(&mut *tx)
        .await
        .map_err(email_write_error)?;
    crate::configuration::reconcile(&mut tx).await?;
    sqlx::query("DELETE FROM sessions WHERE user_id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    bump_revision(&mut tx).await?;
    tx.commit().await?;
    tracing::info!(event="user_updated_sessions_revoked",actor=%p.user.id,user_id=%id);
    Ok(Json(UserInfo {
        id: id.to_string(),
        email,
        name,
        role: input.role,
        active: input.active,
    }))
}
pub async fn reset_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Input(input): Input<PasswordReset>,
) -> Result<StatusCode, AppError> {
    let p = admin(&state, &headers, true).await?;
    let id = uuid(&id)?;
    check_password(&input.password)?;
    let hash = hash_password(input.password).await?;
    let mut tx = begin_admin_checked(&state, &p, input.expected_revision).await?;
    let result = sqlx::query("UPDATE users SET password_hash=$2 WHERE id=$1")
        .bind(id)
        .bind(hash)
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() != 1 {
        return Err(AppError::bad("Unknown user."));
    }
    sqlx::query("DELETE FROM sessions WHERE user_id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    bump_revision(&mut tx).await?;
    tx.commit().await?;
    tracing::info!(event="password_reset_sessions_revoked",actor=%p.user.id,user_id=%id);
    Ok(StatusCode::NO_CONTENT)
}
async fn group_info(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    name: String,
) -> Result<GroupInfo, AppError> {
    let members: Vec<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM memberships WHERE group_id=$1 ORDER BY user_id")
            .bind(id)
            .fetch_all(&mut **tx)
            .await?;
    Ok(GroupInfo {
        id: id.to_string(),
        name,
        member_ids: members.into_iter().map(|id| id.to_string()).collect(),
    })
}
pub async fn list_groups(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<GroupInfo>>, AppError> {
    admin(&state, &headers, false).await?;
    let mut tx = state.pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    let rows: Vec<(Uuid, String)> = sqlx::query_as("SELECT id,name FROM groups ORDER BY name")
        .fetch_all(&mut *tx)
        .await?;
    let mut result = Vec::new();
    for (id, name) in rows {
        result.push(group_info(&mut tx, id, name).await?);
    }
    tx.commit().await?;
    Ok(Json(result))
}
async fn write_members(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    input: &GroupWrite,
) -> Result<(), AppError> {
    if input.member_ids.len() > 10000 {
        return Err(AppError::bad("Too many group members."));
    }
    let ids = input
        .member_ids
        .iter()
        .map(|s| uuid(s))
        .collect::<Result<Vec<_>, _>>()?;
    if ids.iter().collect::<HashSet<_>>().len() != ids.len() {
        return Err(AppError::bad("Duplicate group member."));
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE id=ANY($1)")
        .bind(&ids)
        .fetch_one(&mut **tx)
        .await?;
    if count != ids.len() as i64 {
        return Err(AppError::bad("Unknown group member."));
    }
    sqlx::query("DELETE FROM memberships WHERE group_id=$1")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    sqlx::query("INSERT INTO memberships(group_id,user_id) SELECT $1,unnest($2::uuid[])")
        .bind(id)
        .bind(ids)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
fn group_name(raw: &str) -> Result<String, AppError> {
    let name = raw.trim();
    if name.is_empty() || name.len() > 128 {
        return Err(AppError::bad("Group name must contain 1–128 bytes."));
    }
    Ok(name.into())
}
pub async fn create_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Input(input): Input<GroupWrite>,
) -> Result<Json<GroupInfo>, AppError> {
    let p = admin(&state, &headers, true).await?;
    let name = group_name(&input.name)?;
    let id = Uuid::new_v4();
    let mut tx = begin_admin_checked(&state, &p, input.expected_revision).await?;
    sqlx::query("INSERT INTO groups(id,name) VALUES($1,$2)")
        .bind(id)
        .bind(&name)
        .execute(&mut *tx)
        .await?;
    write_members(&mut tx, id, &input).await?;
    let info = group_info(&mut tx, id, name).await?;
    bump_revision(&mut tx).await?;
    tx.commit().await?;
    tracing::info!(event="group_created",actor=%p.user.id,group_id=%id);
    Ok(Json(info))
}
pub async fn edit_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Input(input): Input<GroupWrite>,
) -> Result<Json<GroupInfo>, AppError> {
    let p = admin(&state, &headers, true).await?;
    let id = uuid(&id)?;
    let name = group_name(&input.name)?;
    let mut tx = begin_admin_checked(&state, &p, input.expected_revision).await?;
    let result = sqlx::query("UPDATE groups SET name=$2 WHERE id=$1")
        .bind(id)
        .bind(&name)
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() != 1 {
        return Err(AppError::bad("Unknown group."));
    }
    write_members(&mut tx, id, &input).await?;
    crate::configuration::reconcile(&mut tx).await?;
    let info = group_info(&mut tx, id, name).await?;
    bump_revision(&mut tx).await?;
    tx.commit().await?;
    tracing::info!(event="group_updated",actor=%p.user.id,group_id=%id);
    Ok(Json(info))
}
pub async fn delete_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Input(input): Input<gateway_core::RevisionInput>,
) -> Result<StatusCode, AppError> {
    let p = admin(&state, &headers, true).await?;
    let id = uuid(&id)?;
    let mut tx = begin_admin_checked(&state, &p, input.expected_revision).await?;
    let result = sqlx::query("DELETE FROM groups WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() != 1 {
        return Err(AppError::bad("Unknown group."));
    }
    crate::configuration::reconcile(&mut tx).await?;
    bump_revision(&mut tx).await?;
    tx.commit().await?;
    tracing::info!(event="group_deleted",actor=%p.user.id,group_id=%id);
    Ok(StatusCode::NO_CONTENT)
}
