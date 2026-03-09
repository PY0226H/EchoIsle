use crate::{AiJudgeJobCreatedEvent, AppError, AppState, JudgeDispatchTrigger};
use chat_core::User;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::FromRow;

mod draw_vote;
mod helpers;
mod report_submit;
mod request_report;
mod types;

pub use types::*;

#[cfg(test)]
use helpers::extract_rag_meta;
use helpers::{
    calc_required_voters, can_request_judge, extract_verdict_evidence_refs, majority_resolution,
    map_draw_vote_detail, map_report_detail, map_stage_summary, normalize_stage_summary_limit,
    normalize_stage_summary_offset, normalize_style_mode, normalize_winner, resolve_rubric_version,
    validate_non_empty_text, validate_score,
};
use sqlx::{Postgres, Transaction};
use tracing::warn;

const STYLE_RATIONAL: &str = "rational";
const STYLE_ENTERTAINING: &str = "entertaining";
const STYLE_MIXED: &str = "mixed";
const STYLE_SOURCE_SYSTEM_CONFIG: &str = "system_config";
const STYLE_SOURCE_SYSTEM_CONFIG_FALLBACK_DEFAULT: &str = "system_config_fallback_default";
const STYLE_SOURCE_EXISTING_RUNNING_JOB: &str = "existing_running_job";
const DRAW_VOTE_THRESHOLD_PERCENT: i32 = 70;
const DRAW_VOTE_WINDOW_SECS: i64 = 300;
const REMATCH_DELAY_SECS: i64 = 600;
const REMATCH_MIN_DURATION_SECS: i64 = 900;
const REMATCH_MAX_DURATION_SECS: i64 = 14_400;
const MAX_STAGE_SUMMARY_COUNT: u32 = 200;
const MAX_STAGE_SUMMARY_OFFSET: u32 = 10_000;
const DEFAULT_OPS_JUDGE_REVIEW_LIMIT: u32 = 50;
const MAX_OPS_JUDGE_REVIEW_LIMIT: u32 = 200;

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
struct AutoJudgeRequesterRow {
    ws_id: i64,
    requester_id: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
struct JudgeJobForUpdate {
    id: i64,
    ws_id: i64,
    session_id: i64,
    status: String,
    rejudge_triggered: bool,
    error_message: Option<String>,
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
    rubric_version: String,
    needs_draw_vote: bool,
    rejudge_triggered: bool,
    payload: Value,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct JudgeStageSummaryRow {
    stage_no: i32,
    from_message_id: Option<i64>,
    to_message_id: Option<i64>,
    pro_score: i32,
    con_score: i32,
    summary: Value,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct SessionMessageEvidenceRow {
    id: i64,
    side: String,
    content: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct JudgeReviewOpsRow {
    report_id: i64,
    session_id: i64,
    job_id: i64,
    winner: String,
    winner_first: Option<String>,
    winner_second: Option<String>,
    pro_score: i32,
    con_score: i32,
    style_mode: String,
    rubric_version: String,
    needs_draw_vote: bool,
    rejudge_triggered: bool,
    verdict_evidence_count: i32,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct DrawVoteRow {
    id: i64,
    ws_id: i64,
    session_id: i64,
    report_id: i64,
    threshold_percent: i32,
    eligible_voters: i32,
    required_voters: i32,
    voting_ends_at: DateTime<Utc>,
    status: String,
    resolution: String,
    decided_at: Option<DateTime<Utc>>,
    rematch_session_id: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
struct DrawVoteStatsRow {
    participated_voters: i32,
    agree_votes: i32,
    disagree_votes: i32,
}

#[derive(Debug, Clone, FromRow)]
struct DebateSessionForRematch {
    id: i64,
    ws_id: i64,
    topic_id: i64,
    scheduled_start_at: DateTime<Utc>,
    actual_start_at: Option<DateTime<Utc>>,
    end_at: DateTime<Utc>,
    max_participants_per_side: i32,
    rematch_round: i32,
}

#[cfg(test)]
mod tests;
