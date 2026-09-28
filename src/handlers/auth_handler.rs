use axum::{extract::State, http::StatusCode, Json};
use chrono::{DateTime, Duration, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    config::tenant::begin_tenant,
    dto::auth_dto::{
        AuthResponse, CurrentUserResponse, ForgotPasswordDto, ForgotPasswordResponse, LoginDto,
        RefreshTokenDto, RegisterDto, ResetPasswordDto,
    },
    error::{ApiError, ApiResult},
    middleware::jwt::AuthUser,
    services::auth_service::{
        generate_opaque_token, generate_token, hash_password, hash_token, verify_password,
        REFRESH_TOKEN_DURATION_DAYS, RESET_TOKEN_DURATION_MINUTES, TOKEN_DURATION_SECONDS,
    },
};

type UserRow = (
    Uuid,
    Uuid,
    String,
    String,
    String,
    bool,
    i32,
    Option<DateTime<Utc>>,
);
type SessionRow = (
    Uuid,
    Uuid,
    Uuid,
    Uuid,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
    String,
    String,
    bool,
);
type ResetRow = (Uuid, Uuid, Uuid, DateTime<Utc>, Option<DateTime<Utc>>);

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterDto>,
) -> ApiResult<(StatusCode, Json<AuthResponse>)> {
    let email = normalize_and_validate(&payload.email, &payload.password, &payload.tenant_name)?;
    let tenant_id = Uuid::new_v4();
    let mut transaction = begin_tenant(&state.db, tenant_id).await?;
    sqlx::query("INSERT INTO tenants (id,name) VALUES ($1,$2)")
        .bind(tenant_id)
        .bind(payload.tenant_name.trim())
        .execute(&mut *transaction)
        .await?;
    let user_id = Uuid::new_v4();
    let role = "ADMIN";
    sqlx::query(
        "INSERT INTO users (id,tenant_id,email,password_hash,role) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(user_id)
    .bind(tenant_id)
    .bind(&email)
    .bind(hash_password(&payload.password)?)
    .bind(role)
    .execute(&mut *transaction)
    .await?;
    let response = issue_session(
        &mut transaction,
        user_id,
        tenant_id,
        email,
        role,
        Uuid::new_v4(),
    )
    .await?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginDto>,
) -> ApiResult<Json<AuthResponse>> {
    let email = payload.email.trim().to_lowercase();
    let user = sqlx::query_as::<_, UserRow>(
        "SELECT id,tenant_id,email,password_hash,role,is_active,failed_login_attempts,locked_until \
         FROM app_private.user_for_login($1)",
    )
    .bind(email)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(invalid_credentials)?;

    if !user.5 {
        return Err(invalid_credentials());
    }
    if user.7.is_some_and(|until| until > Utc::now()) {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "ACCOUNT_LOCKED",
            "Compte temporairement verrouillé",
        ));
    }
    if !verify_password(&user.3, &payload.password) {
        record_failed_login(&state, user.0, user.1).await?;
        return Err(invalid_credentials());
    }

    let mut transaction = begin_tenant(&state.db, user.1).await?;
    sqlx::query("UPDATE users SET failed_login_attempts=0,locked_until=NULL WHERE id=$1")
        .bind(user.0)
        .execute(&mut *transaction)
        .await?;
    let response = issue_session(
        &mut transaction,
        user.0,
        user.1,
        user.2,
        &user.4,
        Uuid::new_v4(),
    )
    .await?;
    transaction.commit().await?;
    Ok(Json(response))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshTokenDto>,
) -> ApiResult<Json<AuthResponse>> {
    let token_hash = hash_token(&payload.refresh_token);
    let session = sqlx::query_as::<_, SessionRow>(
        "SELECT token_id,tenant_id,user_id,family_id,expires_at,revoked_at,email,role,is_active \
         FROM app_private.session_for_refresh($1)",
    )
    .bind(token_hash)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(invalid_refresh_token)?;

    let mut transaction = begin_tenant(&state.db, session.1).await?;
    if session.5.is_some() {
        sqlx::query("UPDATE auth_refresh_tokens SET revoked_at=COALESCE(revoked_at,now()) WHERE family_id=$1")
            .bind(session.3).execute(&mut *transaction).await?;
        transaction.commit().await?;
        return Err(invalid_refresh_token());
    }
    if session.4 <= Utc::now() || !session.8 {
        return Err(invalid_refresh_token());
    }
    sqlx::query("UPDATE auth_refresh_tokens SET revoked_at=now(),last_used_at=now() WHERE id=$1")
        .bind(session.0)
        .execute(&mut *transaction)
        .await?;
    let response = issue_session(
        &mut transaction,
        session.2,
        session.1,
        session.6,
        &session.7,
        session.3,
    )
    .await?;
    transaction.commit().await?;
    Ok(Json(response))
}

pub async fn logout(
    State(state): State<AppState>,
    Json(payload): Json<RefreshTokenDto>,
) -> ApiResult<StatusCode> {
    let session = sqlx::query_as::<_, SessionRow>(
        "SELECT token_id,tenant_id,user_id,family_id,expires_at,revoked_at,email,role,is_active \
         FROM app_private.session_for_refresh($1)",
    )
    .bind(hash_token(&payload.refresh_token))
    .fetch_optional(&state.db)
    .await?;
    if let Some(session) = session {
        let mut transaction = begin_tenant(&state.db, session.1).await?;
        sqlx::query(
            "UPDATE auth_refresh_tokens SET revoked_at=COALESCE(revoked_at,now()) WHERE id=$1",
        )
        .bind(session.0)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn forgot_password(
    State(state): State<AppState>,
    Json(payload): Json<ForgotPasswordDto>,
) -> ApiResult<(StatusCode, Json<ForgotPasswordResponse>)> {
    let user = sqlx::query_as::<_, UserRow>(
        "SELECT id,tenant_id,email,password_hash,role,is_active,failed_login_attempts,locked_until \
         FROM app_private.user_for_login($1)",
    )
    .bind(payload.email.trim().to_lowercase())
    .fetch_optional(&state.db)
    .await?;
    let mut development_reset_token = None;
    if let Some(user) = user.filter(|user| user.5) {
        let token = generate_opaque_token("rst");
        let mut transaction = begin_tenant(&state.db, user.1).await?;
        sqlx::query("UPDATE password_reset_tokens SET used_at=COALESCE(used_at,now()) WHERE user_id=$1 AND used_at IS NULL")
            .bind(user.0).execute(&mut *transaction).await?;
        sqlx::query("INSERT INTO password_reset_tokens (id,tenant_id,user_id,token_hash,expires_at) VALUES ($1,$2,$3,$4,$5)")
            .bind(Uuid::new_v4()).bind(user.1).bind(user.0).bind(hash_token(&token))
            .bind(Utc::now()+Duration::minutes(RESET_TOKEN_DURATION_MINUTES))
            .execute(&mut *transaction).await?;
        transaction.commit().await?;
        if std::env::var("APP_ENV").as_deref() != Ok("production") {
            development_reset_token = Some(token);
        }
    }
    Ok((
        StatusCode::ACCEPTED,
        Json(ForgotPasswordResponse {
            message: "Si le compte existe, les instructions de réinitialisation ont été envoyées",
            development_reset_token,
        }),
    ))
}

pub async fn reset_password(
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordDto>,
) -> ApiResult<StatusCode> {
    validate_password(&payload.new_password)?;
    let reset = sqlx::query_as::<_, ResetRow>(
        "SELECT token_id,tenant_id,user_id,expires_at,used_at FROM app_private.reset_token_for_use($1)",
    )
    .bind(hash_token(&payload.reset_token))
    .fetch_optional(&state.db)
    .await?
    .filter(|row| row.4.is_none() && row.3 > Utc::now())
    .ok_or_else(|| ApiError::bad_request("Jeton de réinitialisation invalide ou expiré"))?;
    let mut transaction = begin_tenant(&state.db, reset.1).await?;
    sqlx::query("UPDATE users SET password_hash=$2,password_changed_at=now(),failed_login_attempts=0,locked_until=NULL WHERE id=$1")
        .bind(reset.2).bind(hash_password(&payload.new_password)?).execute(&mut *transaction).await?;
    sqlx::query("UPDATE password_reset_tokens SET used_at=now() WHERE id=$1 AND used_at IS NULL")
        .bind(reset.0)
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        "UPDATE auth_refresh_tokens SET revoked_at=COALESCE(revoked_at,now()) WHERE user_id=$1",
    )
    .bind(reset.2)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn me(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<CurrentUserResponse>> {
    let mut transaction = begin_tenant(&state.db, auth.tenant_id).await?;
    let row = sqlx::query_as::<_, (Uuid, Uuid, String, String)>(
        "SELECT id,tenant_id,email,role FROM users WHERE id=$1 AND tenant_id=$2 AND is_active",
    )
    .bind(auth.user_id)
    .bind(auth.tenant_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| ApiError::unauthorized("Utilisateur inactif ou introuvable"))?;
    transaction.commit().await?;
    Ok(Json(CurrentUserResponse {
        id: row.0,
        tenant_id: row.1,
        email: row.2,
        role: row.3,
    }))
}

async fn issue_session(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    tenant_id: Uuid,
    email: String,
    role: &str,
    family_id: Uuid,
) -> ApiResult<AuthResponse> {
    let refresh_token = generate_opaque_token("rft");
    sqlx::query("INSERT INTO auth_refresh_tokens (id,tenant_id,user_id,family_id,token_hash,expires_at) VALUES ($1,$2,$3,$4,$5,$6)")
        .bind(Uuid::new_v4()).bind(tenant_id).bind(user_id).bind(family_id)
        .bind(hash_token(&refresh_token)).bind(Utc::now()+Duration::days(REFRESH_TOKEN_DURATION_DAYS))
        .execute(&mut **transaction).await?;
    Ok(AuthResponse {
        token: generate_token(user_id, tenant_id, role)?,
        refresh_token,
        token_type: "Bearer",
        expires_in: TOKEN_DURATION_SECONDS,
        user: CurrentUserResponse {
            id: user_id,
            tenant_id,
            email,
            role: role.to_owned(),
        },
    })
}

async fn record_failed_login(state: &AppState, user_id: Uuid, tenant_id: Uuid) -> ApiResult<()> {
    let mut transaction = begin_tenant(&state.db, tenant_id).await?;
    sqlx::query("UPDATE users SET failed_login_attempts=failed_login_attempts+1,locked_until=CASE WHEN failed_login_attempts+1>=5 THEN now()+interval '15 minutes' ELSE NULL END WHERE id=$1")
        .bind(user_id).execute(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(())
}

fn normalize_and_validate(email: &str, password: &str, tenant: &str) -> ApiResult<String> {
    let email = email.trim().to_lowercase();
    if !email.contains('@') || email.len() > 255 {
        return Err(ApiError::bad_request("Adresse email invalide"));
    }
    validate_password(password)?;
    if tenant.trim().is_empty() || tenant.trim().len() > 255 {
        return Err(ApiError::bad_request("Nom de tenant invalide"));
    }
    Ok(email)
}

fn validate_password(password: &str) -> ApiResult<()> {
    if password.len() < 12 || password.len() > 128 {
        return Err(ApiError::bad_request(
            "Le mot de passe doit contenir entre 12 et 128 caractères",
        ));
    }
    Ok(())
}

fn invalid_credentials() -> ApiError {
    ApiError::unauthorized("Identifiants invalides")
}
fn invalid_refresh_token() -> ApiError {
    ApiError::unauthorized("Refresh token invalide ou expiré")
}
