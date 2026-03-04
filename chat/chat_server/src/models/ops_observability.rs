use crate::{AppError, AppState};
use anyhow::Context;
use chat_core::User;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use std::collections::HashMap;
use utoipa::ToSchema;

use super::OpsPermission;

const MAX_STATE_KEY_LEN: usize = 200;
const MAX_STATE_ITEM_COUNT: usize = 1000;

fn clamp_float(value: f64, min: f64, max: f64, fallback: f64) -> f64 {
    if !value.is_finite() {
        return fallback;
    }
    value.clamp(min, max)
}

fn clamp_int(value: i64, min: i64, max: i64) -> i64 {
    value.clamp(min, max)
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpsObservabilityThresholds {
    pub low_success_rate_threshold: f64,
    pub high_retry_threshold: f64,
    pub high_coalesced_threshold: f64,
    pub high_db_latency_threshold_ms: i64,
    pub low_cache_hit_rate_threshold: f64,
    pub min_request_for_cache_hit_check: i64,
}

impl Default for OpsObservabilityThresholds {
    fn default() -> Self {
        Self {
            low_success_rate_threshold: 80.0,
            high_retry_threshold: 1.0,
            high_coalesced_threshold: 2.0,
            high_db_latency_threshold_ms: 1200,
            low_cache_hit_rate_threshold: 20.0,
            min_request_for_cache_hit_check: 20,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpsObservabilityThresholdsPayload {
    low_success_rate_threshold: Option<f64>,
    high_retry_threshold: Option<f64>,
    high_coalesced_threshold: Option<f64>,
    high_db_latency_threshold_ms: Option<i64>,
    low_cache_hit_rate_threshold: Option<f64>,
    min_request_for_cache_hit_check: Option<i64>,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpsObservabilityAnomalyStateValue {
    #[serde(default)]
    pub acknowledged_at_ms: i64,
    #[serde(default)]
    pub suppress_until_ms: i64,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOpsObservabilityAnomalyStateInput {
    #[serde(default)]
    pub anomaly_state: HashMap<String, OpsObservabilityAnomalyStateValue>,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOpsObservabilityConfigOutput {
    pub ws_id: u64,
    pub thresholds: OpsObservabilityThresholds,
    #[serde(default)]
    pub anomaly_state: HashMap<String, OpsObservabilityAnomalyStateValue>,
    pub updated_by: Option<u64>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
struct OpsObservabilityConfigRow {
    thresholds_json: Value,
    anomaly_state_json: Value,
    updated_by: i64,
    updated_at: DateTime<Utc>,
}

fn normalize_thresholds_payload(
    payload: OpsObservabilityThresholdsPayload,
) -> OpsObservabilityThresholds {
    let defaults = OpsObservabilityThresholds::default();
    OpsObservabilityThresholds {
        low_success_rate_threshold: clamp_float(
            payload
                .low_success_rate_threshold
                .unwrap_or(defaults.low_success_rate_threshold),
            1.0,
            99.99,
            defaults.low_success_rate_threshold,
        ),
        high_retry_threshold: clamp_float(
            payload
                .high_retry_threshold
                .unwrap_or(defaults.high_retry_threshold),
            0.1,
            10.0,
            defaults.high_retry_threshold,
        ),
        high_coalesced_threshold: clamp_float(
            payload
                .high_coalesced_threshold
                .unwrap_or(defaults.high_coalesced_threshold),
            0.1,
            20.0,
            defaults.high_coalesced_threshold,
        ),
        high_db_latency_threshold_ms: clamp_int(
            payload
                .high_db_latency_threshold_ms
                .unwrap_or(defaults.high_db_latency_threshold_ms),
            1,
            60_000,
        ),
        low_cache_hit_rate_threshold: clamp_float(
            payload
                .low_cache_hit_rate_threshold
                .unwrap_or(defaults.low_cache_hit_rate_threshold),
            0.0,
            99.99,
            defaults.low_cache_hit_rate_threshold,
        ),
        min_request_for_cache_hit_check: clamp_int(
            payload
                .min_request_for_cache_hit_check
                .unwrap_or(defaults.min_request_for_cache_hit_check),
            1,
            1_000_000,
        ),
    }
}

fn normalize_thresholds(input: OpsObservabilityThresholds) -> OpsObservabilityThresholds {
    normalize_thresholds_payload(OpsObservabilityThresholdsPayload {
        low_success_rate_threshold: Some(input.low_success_rate_threshold),
        high_retry_threshold: Some(input.high_retry_threshold),
        high_coalesced_threshold: Some(input.high_coalesced_threshold),
        high_db_latency_threshold_ms: Some(input.high_db_latency_threshold_ms),
        low_cache_hit_rate_threshold: Some(input.low_cache_hit_rate_threshold),
        min_request_for_cache_hit_check: Some(input.min_request_for_cache_hit_check),
    })
}

fn parse_thresholds(value: Value) -> OpsObservabilityThresholds {
    let payload = serde_json::from_value::<OpsObservabilityThresholdsPayload>(value)
        .unwrap_or_else(|_| OpsObservabilityThresholdsPayload::default());
    normalize_thresholds_payload(payload)
}

fn now_millis() -> i64 {
    Utc::now().timestamp_millis()
}

fn normalize_anomaly_state(
    input: HashMap<String, OpsObservabilityAnomalyStateValue>,
    now_ms: i64,
) -> HashMap<String, OpsObservabilityAnomalyStateValue> {
    let mut ret = HashMap::new();
    for (key_raw, item) in input {
        if ret.len() >= MAX_STATE_ITEM_COUNT {
            break;
        }
        let key = key_raw.trim();
        if key.is_empty() || key.len() > MAX_STATE_KEY_LEN {
            continue;
        }
        let acknowledged_at_ms = item.acknowledged_at_ms.max(0);
        let suppress_until_ms = if item.suppress_until_ms > now_ms {
            item.suppress_until_ms
        } else {
            0
        };
        if acknowledged_at_ms <= 0 && suppress_until_ms <= 0 {
            continue;
        }
        ret.insert(
            key.to_string(),
            OpsObservabilityAnomalyStateValue {
                acknowledged_at_ms,
                suppress_until_ms,
            },
        );
    }
    ret
}

fn parse_anomaly_state(
    value: Value,
    now_ms: i64,
) -> HashMap<String, OpsObservabilityAnomalyStateValue> {
    let payload =
        serde_json::from_value::<HashMap<String, OpsObservabilityAnomalyStateValue>>(value)
            .unwrap_or_default();
    normalize_anomaly_state(payload, now_ms)
}

fn build_output(
    ws_id: u64,
    row: Option<OpsObservabilityConfigRow>,
    now_ms: i64,
) -> GetOpsObservabilityConfigOutput {
    let Some(row) = row else {
        return GetOpsObservabilityConfigOutput {
            ws_id,
            thresholds: OpsObservabilityThresholds::default(),
            anomaly_state: HashMap::new(),
            updated_by: None,
            updated_at: None,
        };
    };
    GetOpsObservabilityConfigOutput {
        ws_id,
        thresholds: parse_thresholds(row.thresholds_json),
        anomaly_state: parse_anomaly_state(row.anomaly_state_json, now_ms),
        updated_by: Some(row.updated_by as u64),
        updated_at: Some(row.updated_at),
    }
}

impl AppState {
    pub async fn get_ops_observability_config(
        &self,
        user: &User,
    ) -> Result<GetOpsObservabilityConfigOutput, AppError> {
        self.ensure_ops_permission(user, OpsPermission::JudgeReview)
            .await?;
        let row: Option<OpsObservabilityConfigRow> = sqlx::query_as(
            r#"
            SELECT thresholds_json, anomaly_state_json, updated_by, updated_at
            FROM ops_observability_configs
            WHERE ws_id = $1
            "#,
        )
        .bind(user.ws_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(build_output(user.ws_id as u64, row, now_millis()))
    }

    pub async fn upsert_ops_observability_thresholds(
        &self,
        user: &User,
        input: OpsObservabilityThresholds,
    ) -> Result<GetOpsObservabilityConfigOutput, AppError> {
        self.ensure_ops_permission(user, OpsPermission::JudgeReview)
            .await?;
        let thresholds = normalize_thresholds(input);
        let thresholds_json = serde_json::to_value(&thresholds)
            .context("serialize observability thresholds failed")?;
        sqlx::query(
            r#"
            INSERT INTO ops_observability_configs(
                ws_id, thresholds_json, anomaly_state_json, updated_by, created_at, updated_at
            )
            VALUES ($1, $2, '{}'::jsonb, $3, NOW(), NOW())
            ON CONFLICT (ws_id)
            DO UPDATE
            SET thresholds_json = EXCLUDED.thresholds_json,
                updated_by = EXCLUDED.updated_by,
                updated_at = NOW()
            "#,
        )
        .bind(user.ws_id)
        .bind(thresholds_json)
        .bind(user.id)
        .execute(&self.pool)
        .await?;
        self.get_ops_observability_config(user).await
    }

    pub async fn upsert_ops_observability_anomaly_state(
        &self,
        user: &User,
        input: UpdateOpsObservabilityAnomalyStateInput,
    ) -> Result<GetOpsObservabilityConfigOutput, AppError> {
        self.ensure_ops_permission(user, OpsPermission::JudgeReview)
            .await?;
        let now_ms = now_millis();
        let anomaly_state = normalize_anomaly_state(input.anomaly_state, now_ms);
        let anomaly_state_json = serde_json::to_value(&anomaly_state)
            .context("serialize observability anomaly state failed")?;
        let default_thresholds_json =
            serde_json::to_value(OpsObservabilityThresholds::default())
                .context("serialize default observability thresholds failed")?;
        sqlx::query(
            r#"
            INSERT INTO ops_observability_configs(
                ws_id, thresholds_json, anomaly_state_json, updated_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            ON CONFLICT (ws_id)
            DO UPDATE
            SET anomaly_state_json = EXCLUDED.anomaly_state_json,
                updated_by = EXCLUDED.updated_by,
                updated_at = NOW()
            "#,
        )
        .bind(user.ws_id)
        .bind(default_thresholds_json)
        .bind(anomaly_state_json)
        .bind(user.id)
        .execute(&self.pool)
        .await?;
        self.get_ops_observability_config(user).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UpsertOpsRoleInput;
    use anyhow::Result;

    #[tokio::test]
    async fn get_ops_observability_config_should_return_defaults_when_missing() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        state.update_workspace_owner(1, 1).await?;
        let owner = state.find_user_by_id(1).await?.expect("owner should exist");
        let ret = state.get_ops_observability_config(&owner).await?;
        assert_eq!(ret.ws_id, 1);
        assert_eq!(ret.thresholds.low_success_rate_threshold, 80.0);
        assert!(ret.anomaly_state.is_empty());
        assert!(ret.updated_by.is_none());
        assert!(ret.updated_at.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn upsert_ops_observability_config_should_allow_ops_viewer_review_permission(
    ) -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        state.update_workspace_owner(1, 1).await?;
        let owner = state.find_user_by_id(1).await?.expect("owner should exist");
        let viewer = state
            .find_user_by_id(2)
            .await?
            .expect("viewer should exist");
        state
            .upsert_ops_role_assignment_by_owner(
                &owner,
                viewer.id as u64,
                UpsertOpsRoleInput {
                    role: "ops_viewer".to_string(),
                },
            )
            .await?;

        let threshold_ret = state
            .upsert_ops_observability_thresholds(
                &viewer,
                OpsObservabilityThresholds {
                    low_success_rate_threshold: 75.0,
                    high_retry_threshold: 1.5,
                    high_coalesced_threshold: 2.5,
                    high_db_latency_threshold_ms: 1500,
                    low_cache_hit_rate_threshold: 25.0,
                    min_request_for_cache_hit_check: 30,
                },
            )
            .await?;
        assert_eq!(threshold_ret.updated_by, Some(viewer.id as u64));
        assert_eq!(threshold_ret.thresholds.low_success_rate_threshold, 75.0);

        let state_ret = state
            .upsert_ops_observability_anomaly_state(
                &viewer,
                UpdateOpsObservabilityAnomalyStateInput {
                    anomaly_state: HashMap::from([(
                        "db_errors".to_string(),
                        OpsObservabilityAnomalyStateValue {
                            acknowledged_at_ms: 1000,
                            suppress_until_ms: now_millis() + 60_000,
                        },
                    )]),
                },
            )
            .await?;
        assert_eq!(state_ret.updated_by, Some(viewer.id as u64));
        assert!(state_ret.anomaly_state.contains_key("db_errors"));
        assert_eq!(
            state_ret
                .anomaly_state
                .get("db_errors")
                .map(|item| item.acknowledged_at_ms),
            Some(1000)
        );
        Ok(())
    }

    #[tokio::test]
    async fn upsert_ops_observability_config_should_reject_user_without_ops_role() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        state.update_workspace_owner(1, 1).await?;
        let user = state.find_user_by_id(3).await?.expect("user should exist");
        let err = state
            .upsert_ops_observability_thresholds(&user, OpsObservabilityThresholds::default())
            .await
            .expect_err("missing ops role should be rejected");
        match err {
            AppError::DebateConflict(msg) => {
                assert!(msg.starts_with("ops_permission_denied:judge_review:"));
            }
            other => panic!("unexpected error: {}", other),
        }
        Ok(())
    }
}
