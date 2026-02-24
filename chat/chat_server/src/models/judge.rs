use crate::{AiJudgeJobCreatedEvent, AppError, AppState};
use chat_core::User;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use tracing::warn;
use utoipa::ToSchema;

const STYLE_RATIONAL: &str = "rational";
const STYLE_ENTERTAINING: &str = "entertaining";
const STYLE_MIXED: &str = "mixed";

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestJudgeJobInput {
    pub style_mode: Option<String>,
    #[serde(default)]
    pub allow_rejudge: bool,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestJudgeJobOutput {
    pub session_id: u64,
    pub job_id: u64,
    pub status: String,
    pub style_mode: String,
    pub rejudge_triggered: bool,
    pub requested_at: DateTime<Utc>,
    pub newly_created: bool,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JudgeJobSnapshot {
    pub job_id: u64,
    pub status: String,
    pub style_mode: String,
    pub rejudge_triggered: bool,
    pub requested_at: DateTime<Utc>,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JudgeReportDetail {
    pub report_id: u64,
    pub job_id: u64,
    pub winner: String,
    pub pro_score: i32,
    pub con_score: i32,
    pub logic_pro: i32,
    pub logic_con: i32,
    pub evidence_pro: i32,
    pub evidence_con: i32,
    pub rebuttal_pro: i32,
    pub rebuttal_con: i32,
    pub clarity_pro: i32,
    pub clarity_con: i32,
    pub pro_summary: String,
    pub con_summary: String,
    pub rationale: String,
    pub style_mode: String,
    pub needs_draw_vote: bool,
    pub rejudge_triggered: bool,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetJudgeReportOutput {
    pub session_id: u64,
    pub status: String,
    pub latest_job: Option<JudgeJobSnapshot>,
    pub report: Option<JudgeReportDetail>,
}

#[derive(Debug, Clone, FromRow)]
struct DebateSessionForJudge {
    ws_id: i64,
    status: String,
}

#[derive(Debug, Clone, FromRow)]
struct JudgeJobRow {
    id: i64,
    status: String,
    style_mode: String,
    rejudge_triggered: bool,
    requested_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct JudgeReportRow {
    id: i64,
    job_id: i64,
    winner: String,
    pro_score: i32,
    con_score: i32,
    logic_pro: i32,
    logic_con: i32,
    evidence_pro: i32,
    evidence_con: i32,
    rebuttal_pro: i32,
    rebuttal_con: i32,
    clarity_pro: i32,
    clarity_con: i32,
    pro_summary: String,
    con_summary: String,
    rationale: String,
    style_mode: String,
    needs_draw_vote: bool,
    rejudge_triggered: bool,
    payload: Value,
    created_at: DateTime<Utc>,
}

fn normalize_style_mode(style_mode: Option<String>) -> Result<String, AppError> {
    let raw = style_mode.unwrap_or_else(|| STYLE_RATIONAL.to_string());
    let mode = raw.trim().to_ascii_lowercase();
    if mode.is_empty() {
        return Err(AppError::DebateError(
            "style_mode cannot be empty".to_string(),
        ));
    }
    if matches!(
        mode.as_str(),
        STYLE_RATIONAL | STYLE_ENTERTAINING | STYLE_MIXED
    ) {
        Ok(mode)
    } else {
        Err(AppError::DebateError(format!(
            "invalid style_mode: {raw}, expect `rational` | `entertaining` | `mixed`"
        )))
    }
}

fn can_request_judge(status: &str) -> bool {
    matches!(status, "judging" | "closed")
}

impl AppState {
    pub async fn request_judge_job(
        &self,
        session_id: u64,
        user: &User,
        input: RequestJudgeJobInput,
    ) -> Result<RequestJudgeJobOutput, AppError> {
        let style_mode = normalize_style_mode(input.style_mode)?;
        let mut tx = self.pool.begin().await?;

        let Some(session): Option<DebateSessionForJudge> = sqlx::query_as(
            r#"
            SELECT ws_id, status
            FROM debate_sessions
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(session_id as i64)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Err(AppError::NotFound(format!(
                "debate session id {session_id}"
            )));
        };

        if session.ws_id != user.ws_id {
            return Err(AppError::NotFound(format!(
                "debate session id {session_id}"
            )));
        }

        if !can_request_judge(&session.status) {
            return Err(AppError::DebateConflict(format!(
                "session {} is not in judging/closed status",
                session_id
            )));
        }

        let joined: Option<(i32,)> = sqlx::query_as(
            r#"
            SELECT 1
            FROM session_participants
            WHERE session_id = $1 AND user_id = $2
            "#,
        )
        .bind(session_id as i64)
        .bind(user.id)
        .fetch_optional(&mut *tx)
        .await?;
        if joined.is_none() {
            return Err(AppError::DebateConflict(format!(
                "user {} has not joined session {}",
                user.id, session_id
            )));
        }

        let existing_running: Option<JudgeJobRow> = sqlx::query_as(
            r#"
            SELECT id, status, style_mode, rejudge_triggered, requested_at
            FROM judge_jobs
            WHERE session_id = $1 AND status = 'running'
            ORDER BY requested_at DESC
            LIMIT 1
            "#,
        )
        .bind(session_id as i64)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(job) = existing_running {
            tx.commit().await?;
            return Ok(RequestJudgeJobOutput {
                session_id,
                job_id: job.id as u64,
                status: job.status,
                style_mode: job.style_mode,
                rejudge_triggered: job.rejudge_triggered,
                requested_at: job.requested_at,
                newly_created: false,
            });
        }

        let report_exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT id
            FROM judge_reports
            WHERE session_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(session_id as i64)
        .fetch_optional(&mut *tx)
        .await?
        .is_some();
        if report_exists && !input.allow_rejudge {
            return Err(AppError::DebateConflict(format!(
                "session {} already has judge report, set allowRejudge=true to create rejudge job",
                session_id
            )));
        }

        let rejudge_triggered = report_exists && input.allow_rejudge;
        let job: JudgeJobRow = sqlx::query_as(
            r#"
            INSERT INTO judge_jobs(
                ws_id, session_id, requested_by, status, style_mode, rejudge_triggered,
                requested_at, started_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, 'running', $4, $5, NOW(), NOW(), NOW(), NOW())
            RETURNING id, status, style_mode, rejudge_triggered, requested_at
            "#,
        )
        .bind(user.ws_id)
        .bind(session_id as i64)
        .bind(user.id)
        .bind(&style_mode)
        .bind(rejudge_triggered)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        if let Err(err) = self
            .event_bus
            .publish_ai_judge_job_created(AiJudgeJobCreatedEvent {
                ws_id: user.ws_id as u64,
                session_id,
                job_id: job.id as u64,
                requested_by: user.id as u64,
                style_mode: style_mode.clone(),
                rejudge_triggered,
                requested_at: job.requested_at,
            })
            .await
        {
            warn!(
                session_id,
                user_id = user.id,
                "publish kafka ai judge job created failed: {}",
                err
            );
        }

        Ok(RequestJudgeJobOutput {
            session_id,
            job_id: job.id as u64,
            status: job.status,
            style_mode: job.style_mode,
            rejudge_triggered: job.rejudge_triggered,
            requested_at: job.requested_at,
            newly_created: true,
        })
    }

    pub async fn get_latest_judge_report(
        &self,
        session_id: u64,
        user: &User,
    ) -> Result<GetJudgeReportOutput, AppError> {
        let session_ws_id: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT ws_id
            FROM debate_sessions
            WHERE id = $1
            "#,
        )
        .bind(session_id as i64)
        .fetch_optional(&self.pool)
        .await?;
        let Some((session_ws_id,)) = session_ws_id else {
            return Err(AppError::NotFound(format!(
                "debate session id {session_id}"
            )));
        };
        if session_ws_id != user.ws_id {
            return Err(AppError::NotFound(format!(
                "debate session id {session_id}"
            )));
        }

        let latest_job: Option<JudgeJobRow> = sqlx::query_as(
            r#"
            SELECT id, status, style_mode, rejudge_triggered, requested_at
            FROM judge_jobs
            WHERE session_id = $1
            ORDER BY requested_at DESC
            LIMIT 1
            "#,
        )
        .bind(session_id as i64)
        .fetch_optional(&self.pool)
        .await?;

        let report: Option<JudgeReportRow> = sqlx::query_as(
            r#"
            SELECT
                id, job_id, winner, pro_score, con_score,
                logic_pro, logic_con, evidence_pro, evidence_con, rebuttal_pro, rebuttal_con,
                clarity_pro, clarity_con, pro_summary, con_summary, rationale, style_mode,
                needs_draw_vote, rejudge_triggered, payload, created_at
            FROM judge_reports
            WHERE session_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(session_id as i64)
        .fetch_optional(&self.pool)
        .await?;

        let status = if report.is_some() {
            "ready".to_string()
        } else if let Some(job) = latest_job.as_ref() {
            if job.status == "failed" {
                "failed".to_string()
            } else {
                "pending".to_string()
            }
        } else {
            "absent".to_string()
        };

        Ok(GetJudgeReportOutput {
            session_id,
            status,
            latest_job: latest_job.map(|job| JudgeJobSnapshot {
                job_id: job.id as u64,
                status: job.status,
                style_mode: job.style_mode,
                rejudge_triggered: job.rejudge_triggered,
                requested_at: job.requested_at,
            }),
            report: report.map(|v| JudgeReportDetail {
                report_id: v.id as u64,
                job_id: v.job_id as u64,
                winner: v.winner,
                pro_score: v.pro_score,
                con_score: v.con_score,
                logic_pro: v.logic_pro,
                logic_con: v.logic_con,
                evidence_pro: v.evidence_pro,
                evidence_con: v.evidence_con,
                rebuttal_pro: v.rebuttal_pro,
                rebuttal_con: v.rebuttal_con,
                clarity_pro: v.clarity_pro,
                clarity_con: v.clarity_con,
                pro_summary: v.pro_summary,
                con_summary: v.con_summary,
                rationale: v.rationale,
                style_mode: v.style_mode,
                needs_draw_vote: v.needs_draw_vote,
                rejudge_triggered: v.rejudge_triggered,
                payload: v.payload,
                created_at: v.created_at,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use chrono::Duration;

    async fn seed_topic_and_session(state: &AppState, ws_id: i64, status: &str) -> Result<i64> {
        let topic_id: (i64,) = sqlx::query_as(
            r#"
            INSERT INTO debate_topics(ws_id, title, description, category, stance_pro, stance_con, is_active, created_by)
            VALUES ($1, 'topic-ai', 'desc', 'game', 'pro', 'con', true, 1)
            RETURNING id
            "#,
        )
        .bind(ws_id)
        .fetch_one(&state.pool)
        .await?;

        let now = Utc::now();
        let session_id: (i64,) = sqlx::query_as(
            r#"
            INSERT INTO debate_sessions(
                ws_id, topic_id, status, scheduled_start_at, actual_start_at, end_at, max_participants_per_side
            )
            VALUES ($1, $2, $3, $4, $5, $6, 500)
            RETURNING id
            "#,
        )
        .bind(ws_id)
        .bind(topic_id.0)
        .bind(status)
        .bind(now - Duration::minutes(20))
        .bind(now - Duration::minutes(15))
        .bind(now - Duration::minutes(1))
        .fetch_one(&state.pool)
        .await?;

        Ok(session_id.0)
    }

    async fn join_user_to_session(state: &AppState, session_id: i64, user_id: i64) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO session_participants(session_id, user_id, side)
            VALUES ($1, $2, 'pro')
            "#,
        )
        .bind(session_id)
        .bind(user_id)
        .execute(&state.pool)
        .await?;
        Ok(())
    }

    #[tokio::test]
    async fn request_judge_job_should_create_running_job_with_default_style() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let session_id = seed_topic_and_session(&state, 1, "judging").await?;
        join_user_to_session(&state, session_id, 1).await?;
        let user = state.find_user_by_id(1).await?.expect("user should exist");

        let ret = state
            .request_judge_job(
                session_id as u64,
                &user,
                RequestJudgeJobInput {
                    style_mode: None,
                    allow_rejudge: false,
                },
            )
            .await?;

        assert!(ret.newly_created);
        assert_eq!(ret.style_mode, "rational");
        assert_eq!(ret.status, "running");
        Ok(())
    }

    #[tokio::test]
    async fn request_judge_job_should_be_idempotent_with_running_job() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let session_id = seed_topic_and_session(&state, 1, "closed").await?;
        join_user_to_session(&state, session_id, 1).await?;
        let user = state.find_user_by_id(1).await?.expect("user should exist");

        let first = state
            .request_judge_job(
                session_id as u64,
                &user,
                RequestJudgeJobInput {
                    style_mode: Some("mixed".to_string()),
                    allow_rejudge: false,
                },
            )
            .await?;
        let second = state
            .request_judge_job(
                session_id as u64,
                &user,
                RequestJudgeJobInput {
                    style_mode: Some("mixed".to_string()),
                    allow_rejudge: false,
                },
            )
            .await?;

        assert!(first.newly_created);
        assert!(!second.newly_created);
        assert_eq!(first.job_id, second.job_id);
        Ok(())
    }

    #[tokio::test]
    async fn request_judge_job_should_reject_user_not_joined() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let session_id = seed_topic_and_session(&state, 1, "judging").await?;
        let user = state.find_user_by_id(1).await?.expect("user should exist");

        let err = state
            .request_judge_job(
                session_id as u64,
                &user,
                RequestJudgeJobInput {
                    style_mode: None,
                    allow_rejudge: false,
                },
            )
            .await
            .expect_err("should reject non participant");
        assert!(matches!(err, AppError::DebateConflict(_)));
        Ok(())
    }

    #[tokio::test]
    async fn request_judge_job_should_reject_non_judging_session() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let session_id = seed_topic_and_session(&state, 1, "running").await?;
        join_user_to_session(&state, session_id, 1).await?;
        let user = state.find_user_by_id(1).await?.expect("user should exist");

        let err = state
            .request_judge_job(
                session_id as u64,
                &user,
                RequestJudgeJobInput {
                    style_mode: None,
                    allow_rejudge: false,
                },
            )
            .await
            .expect_err("running status should reject");
        assert!(matches!(err, AppError::DebateConflict(_)));
        Ok(())
    }

    #[tokio::test]
    async fn get_latest_judge_report_should_return_pending_when_job_running() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let session_id = seed_topic_and_session(&state, 1, "judging").await?;
        join_user_to_session(&state, session_id, 1).await?;
        let user = state.find_user_by_id(1).await?.expect("user should exist");

        state
            .request_judge_job(
                session_id as u64,
                &user,
                RequestJudgeJobInput {
                    style_mode: None,
                    allow_rejudge: false,
                },
            )
            .await?;

        let ret = state
            .get_latest_judge_report(session_id as u64, &user)
            .await?;
        assert_eq!(ret.status, "pending");
        assert!(ret.latest_job.is_some());
        assert!(ret.report.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn get_latest_judge_report_should_return_ready_when_report_exists() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let session_id = seed_topic_and_session(&state, 1, "closed").await?;
        join_user_to_session(&state, session_id, 1).await?;
        let user = state.find_user_by_id(1).await?.expect("user should exist");

        let job_id: (i64,) = sqlx::query_as(
            r#"
            INSERT INTO judge_jobs(
                ws_id, session_id, requested_by, status, style_mode, requested_at, started_at, finished_at
            )
            VALUES ($1, $2, $3, 'succeeded', 'rational', NOW(), NOW(), NOW())
            RETURNING id
            "#,
        )
        .bind(1_i64)
        .bind(session_id)
        .bind(1_i64)
        .fetch_one(&state.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO judge_reports(
                ws_id, session_id, job_id, winner, pro_score, con_score,
                logic_pro, logic_con, evidence_pro, evidence_con, rebuttal_pro, rebuttal_con,
                clarity_pro, clarity_con, pro_summary, con_summary, rationale, style_mode,
                needs_draw_vote, rejudge_triggered, payload
            )
            VALUES (
                $1, $2, $3, 'pro', 82, 74,
                80, 72, 85, 76, 79, 71,
                84, 77, 'pro summary', 'con summary', 'rationale', 'rational',
                false, false, '{"trace":"ok"}'::jsonb
            )
            "#,
        )
        .bind(1_i64)
        .bind(session_id)
        .bind(job_id.0)
        .execute(&state.pool)
        .await?;

        let ret = state
            .get_latest_judge_report(session_id as u64, &user)
            .await?;
        assert_eq!(ret.status, "ready");
        let report = ret.report.expect("report should exist");
        assert_eq!(report.job_id, job_id.0 as u64);
        assert_eq!(report.winner, "pro");
        assert_eq!(report.pro_score, 82);
        Ok(())
    }
}
