use crate::{
    AppError, AppState, CreateDebateMessageInput, JoinDebateSessionInput, ListDebateSessions,
    ListDebateTopics, PinDebateMessageInput, RequestJudgeJobInput, SubmitDrawVoteInput,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use chat_core::User;

/// List debate topics in the current workspace.
#[utoipa::path(
    get,
    path = "/api/debate/topics",
    params(
        ListDebateTopics
    ),
    responses(
        (status = 200, description = "List of debate topics", body = Vec<crate::DebateTopic>),
    ),
    security(
        ("token" = [])
    )
)]
pub(crate) async fn list_debate_topics_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Query(input): Query<ListDebateTopics>,
) -> Result<impl IntoResponse, AppError> {
    let topics = state.list_debate_topics(user.ws_id as _, input).await?;
    Ok((StatusCode::OK, Json(topics)))
}

/// List debate sessions in the current workspace.
#[utoipa::path(
    get,
    path = "/api/debate/sessions",
    params(
        ListDebateSessions
    ),
    responses(
        (status = 200, description = "List of debate sessions", body = Vec<crate::DebateSessionSummary>),
    ),
    security(
        ("token" = [])
    )
)]
pub(crate) async fn list_debate_sessions_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Query(input): Query<ListDebateSessions>,
) -> Result<impl IntoResponse, AppError> {
    let sessions = state.list_debate_sessions(user.ws_id as _, input).await?;
    Ok((StatusCode::OK, Json(sessions)))
}

/// Join a debate session with selected side.
#[utoipa::path(
    post,
    path = "/api/debate/sessions/{id}/join",
    params(
        ("id" = u64, Path, description = "Debate session id")
    ),
    request_body = JoinDebateSessionInput,
    responses(
        (status = 200, description = "Join result", body = crate::JoinDebateSessionOutput),
        (status = 400, description = "Invalid input", body = ErrorOutput),
        (status = 404, description = "Debate session not found", body = ErrorOutput),
        (status = 409, description = "Join conflict", body = ErrorOutput),
    ),
    security(
        ("token" = [])
    )
)]
pub(crate) async fn join_debate_session_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(input): Json<JoinDebateSessionInput>,
) -> Result<impl IntoResponse, AppError> {
    let result = state.join_debate_session(id, &user, input).await?;
    Ok((StatusCode::OK, Json(result)))
}

/// Send a message in a debate session.
#[utoipa::path(
    post,
    path = "/api/debate/sessions/{id}/messages",
    params(
        ("id" = u64, Path, description = "Debate session id")
    ),
    request_body = CreateDebateMessageInput,
    responses(
        (status = 201, description = "Created message", body = crate::DebateMessage),
        (status = 400, description = "Invalid input", body = ErrorOutput),
        (status = 404, description = "Debate session not found", body = ErrorOutput),
        (status = 409, description = "Session conflict", body = ErrorOutput),
    ),
    security(
        ("token" = [])
    )
)]
pub(crate) async fn create_debate_message_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(input): Json<CreateDebateMessageInput>,
) -> Result<impl IntoResponse, AppError> {
    let msg = state.create_debate_message(id, &user, input).await?;
    Ok((StatusCode::CREATED, Json(msg)))
}

/// Pin an existing debate message with wallet consume.
#[utoipa::path(
    post,
    path = "/api/debate/messages/{id}/pin",
    params(
        ("id" = u64, Path, description = "Debate message id")
    ),
    request_body = PinDebateMessageInput,
    responses(
        (status = 200, description = "Pin result", body = crate::PinDebateMessageOutput),
        (status = 400, description = "Invalid input", body = ErrorOutput),
        (status = 404, description = "Debate message not found", body = ErrorOutput),
        (status = 409, description = "Pin conflict", body = ErrorOutput),
    ),
    security(
        ("token" = [])
    )
)]
pub(crate) async fn pin_debate_message_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(input): Json<PinDebateMessageInput>,
) -> Result<impl IntoResponse, AppError> {
    let ret = state.pin_debate_message(id, &user, input).await?;
    Ok((StatusCode::OK, Json(ret)))
}

/// Request an AI judge job for a debate session.
/// Note: `styleMode` in request body is kept for compatibility and no longer controls behavior.
/// Effective style is decided by server-side `ai_judge.style_mode` config and returned in `styleModeSource`.
#[utoipa::path(
    post,
    path = "/api/debate/sessions/{id}/judge/jobs",
    params(
        ("id" = u64, Path, description = "Debate session id")
    ),
    request_body = RequestJudgeJobInput,
    responses(
        (status = 202, description = "Judge job accepted", body = crate::RequestJudgeJobOutput),
        (status = 400, description = "Invalid input", body = ErrorOutput),
        (status = 404, description = "Debate session not found", body = ErrorOutput),
        (status = 409, description = "Request conflict", body = ErrorOutput),
    ),
    security(
        ("token" = [])
    )
)]
pub(crate) async fn request_judge_job_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(input): Json<RequestJudgeJobInput>,
) -> Result<impl IntoResponse, AppError> {
    let ret = state.request_judge_job(id, &user, input).await?;
    Ok((StatusCode::ACCEPTED, Json(ret)))
}

/// Get latest AI judge report for a debate session.
#[utoipa::path(
    get,
    path = "/api/debate/sessions/{id}/judge-report",
    params(
        ("id" = u64, Path, description = "Debate session id")
    ),
    responses(
        (status = 200, description = "Judge report query result", body = crate::GetJudgeReportOutput),
        (status = 404, description = "Debate session not found", body = ErrorOutput),
    ),
    security(
        ("token" = [])
    )
)]
pub(crate) async fn get_latest_judge_report_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse, AppError> {
    let ret = state.get_latest_judge_report(id, &user).await?;
    Ok((StatusCode::OK, Json(ret)))
}

/// Get draw-vote status for latest draw-required judge report in a debate session.
#[utoipa::path(
    get,
    path = "/api/debate/sessions/{id}/draw-vote",
    params(
        ("id" = u64, Path, description = "Debate session id")
    ),
    responses(
        (status = 200, description = "Draw vote status", body = crate::GetDrawVoteOutput),
        (status = 404, description = "Debate session not found", body = ErrorOutput),
        (status = 409, description = "User is not participant", body = ErrorOutput),
    ),
    security(
        ("token" = [])
    )
)]
pub(crate) async fn get_draw_vote_status_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse, AppError> {
    let ret = state.get_draw_vote_status(id, &user).await?;
    Ok((StatusCode::OK, Json(ret)))
}

/// Submit or update current user's draw vote.
#[utoipa::path(
    post,
    path = "/api/debate/sessions/{id}/draw-vote/ballots",
    params(
        ("id" = u64, Path, description = "Debate session id")
    ),
    request_body = SubmitDrawVoteInput,
    responses(
        (status = 200, description = "Draw vote submit result", body = crate::SubmitDrawVoteOutput),
        (status = 400, description = "Invalid input", body = ErrorOutput),
        (status = 404, description = "Debate session not found", body = ErrorOutput),
        (status = 409, description = "Vote conflict", body = ErrorOutput),
    ),
    security(
        ("token" = [])
    )
)]
pub(crate) async fn submit_draw_vote_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(input): Json<SubmitDrawVoteInput>,
) -> Result<impl IntoResponse, AppError> {
    let ret = state.submit_draw_vote(id, &user, input).await?;
    Ok((StatusCode::OK, Json(ret)))
}
