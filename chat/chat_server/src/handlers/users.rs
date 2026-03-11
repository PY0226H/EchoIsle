use crate::{AppError, AppState};
use axum::{extract::State, response::IntoResponse, Extension, Json};
use chat_core::User;

/// List all users in the platform scope.
#[utoipa::path(
    get,
    path = "/api/users",
    responses(
        (status = 200, description = "List of users", body = Vec<ChatUser>),
    ),
    security(
        ("token" = [])
    )
)]
pub(crate) async fn list_chat_users_handler(
    Extension(_user): Extension<User>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let users = state.fetch_chat_users().await?;
    Ok(Json(users))
}
