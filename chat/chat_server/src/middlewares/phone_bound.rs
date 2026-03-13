use crate::{AppError, AppState, ErrorOutput};
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use chat_core::User;

pub async fn require_phone_bound(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let (parts, body) = req.into_parts();
    let Some(user) = parts.extensions.get::<User>() else {
        return AppError::NotLoggedIn.into_response();
    };

    let current_user = match state.find_user_by_id(user.id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorOutput::new("auth_access_invalid")),
            )
                .into_response();
        }
        Err(err) => {
            tracing::warn!(
                user_id = user.id,
                "load user profile failed in phone gate: {}",
                err
            );
            return err.into_response();
        }
    };

    let has_bound_phone = current_user
        .phone_e164
        .as_deref()
        .map(str::trim)
        .filter(|phone| !phone.is_empty())
        .is_some();
    if current_user.phone_bind_required || !has_bound_phone {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorOutput::new("auth_phone_bind_required")),
        )
            .into_response();
    }

    let mut req = Request::from_parts(parts, body);
    req.extensions_mut().insert(current_user);
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use axum::{
        body::Body, http::StatusCode, middleware::from_fn_with_state, response::IntoResponse,
        routing::get, Router,
    };
    use chat_core::middlewares::verify_token;
    use tower::ServiceExt;

    async fn handler(_req: Request) -> impl IntoResponse {
        (StatusCode::OK, "ok")
    }

    async fn issue_token_for_user(
        state: &AppState,
        user_id: i64,
        sid: &str,
    ) -> Result<String, AppError> {
        let family_id = format!("{sid}-family");
        let refresh_jti = format!("{sid}-refresh-jti");
        let access_jti = format!("{sid}-access-jti");

        sqlx::query(
            r#"
            INSERT INTO auth_refresh_sessions (
                user_id, sid, family_id, current_jti, expires_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, NOW() + interval '1 day', NOW(), NOW())
            ON CONFLICT (sid) DO UPDATE
            SET current_jti = EXCLUDED.current_jti,
                family_id = EXCLUDED.family_id,
                revoked_at = NULL,
                revoke_reason = NULL,
                expires_at = EXCLUDED.expires_at,
                updated_at = NOW()
            "#,
        )
        .bind(user_id)
        .bind(sid)
        .bind(&family_id)
        .bind(&refresh_jti)
        .execute(&state.pool)
        .await?;

        let token = state
            .ek
            .sign_access_token_with_jti(user_id, sid, 0, access_jti, 900)?;
        Ok(token)
    }

    #[tokio::test]
    async fn require_phone_bound_should_allow_bound_user() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let user = state
            .find_user_by_id(1)
            .await?
            .expect("seed user should exist");
        let token = issue_token_for_user(&state, user.id, "phone-gate-allow")
            .await
            .expect("issue token");

        let app = Router::new()
            .route("/protected", get(handler))
            .layer(from_fn_with_state(state.clone(), require_phone_bound))
            .layer(from_fn_with_state(state.clone(), verify_token::<AppState>))
            .with_state(state);

        let req = Request::builder()
            .uri("/protected")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;
        let res = app.oneshot(req).await?;
        assert_eq!(res.status(), StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn require_phone_bound_should_reject_unbound_user() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let user = state
            .create_user(&crate::models::CreateUser {
                fullname: "No Phone".to_string(),
                email: "no-phone@acme.org".to_string(),
                password: "123456".to_string(),
            })
            .await?;
        let token = issue_token_for_user(&state, user.id, "phone-gate-reject")
            .await
            .expect("issue token");

        let app = Router::new()
            .route("/protected", get(handler))
            .layer(from_fn_with_state(state.clone(), require_phone_bound))
            .layer(from_fn_with_state(state.clone(), verify_token::<AppState>))
            .with_state(state);

        let req = Request::builder()
            .uri("/protected")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;
        let res = app.oneshot(req).await?;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        Ok(())
    }
}
