use crate::{AppError, AppState, MarkJudgeJobFailedInput, SubmitJudgeReportInput};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

/// Internal callback for AI service to persist judge report.
#[utoipa::path(
    post,
    path = "/api/internal/ai/judge/jobs/{id}/report",
    params(
        ("id" = u64, Path, description = "Judge job id")
    ),
    request_body = SubmitJudgeReportInput,
    responses(
        (status = 200, description = "Judge report persisted", body = crate::SubmitJudgeReportOutput),
        (status = 400, description = "Invalid input", body = ErrorOutput),
        (status = 404, description = "Judge job not found", body = ErrorOutput),
        (status = 409, description = "Job state conflict", body = ErrorOutput),
    ),
    security(
        ("internal_key" = [])
    )
)]
pub(crate) async fn submit_judge_report_handler(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(input): Json<SubmitJudgeReportInput>,
) -> Result<impl IntoResponse, AppError> {
    let ret = state.submit_judge_report(id, input).await?;
    Ok((StatusCode::OK, Json(ret)))
}

/// Internal callback for AI service to mark a judge job as failed.
#[utoipa::path(
    post,
    path = "/api/internal/ai/judge/jobs/{id}/failed",
    params(
        ("id" = u64, Path, description = "Judge job id")
    ),
    request_body = MarkJudgeJobFailedInput,
    responses(
        (status = 200, description = "Judge job marked failed", body = crate::MarkJudgeJobFailedOutput),
        (status = 400, description = "Invalid input", body = ErrorOutput),
        (status = 404, description = "Judge job not found", body = ErrorOutput),
        (status = 409, description = "Job state conflict", body = ErrorOutput),
    ),
    security(
        ("internal_key" = [])
    )
)]
pub(crate) async fn mark_judge_job_failed_handler(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(input): Json<MarkJudgeJobFailedInput>,
) -> Result<impl IntoResponse, AppError> {
    let ret = state.mark_judge_job_failed(id, input).await?;
    Ok((StatusCode::OK, Json(ret)))
}
