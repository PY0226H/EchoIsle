use std::{sync::Arc, time::Duration};

use anyhow::Context;
use chrono::{DateTime, Utc};
use rdkafka::{
    consumer::{CommitMode, Consumer, StreamConsumer},
    producer::{FutureProducer, FutureRecord},
    util::Timeout,
    ClientConfig, Message,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::config::KafkaConfig;

pub const TOPIC_DEBATE_PARTICIPANT_JOINED: &str = "debate.participant.joined.v1";
pub const TOPIC_DEBATE_SESSION_STATUS_CHANGED: &str = "debate.session.status.changed.v1";
pub const TOPIC_DEBATE_MESSAGE_PINNED: &str = "debate.message.pinned.v1";
pub const TOPIC_AI_JUDGE_JOB_CREATED: &str = "ai.judge.job.created.v1";
const LEDGER_STATUS_SUCCEEDED: &str = "succeeded";
const LEDGER_STATUS_FAILED: &str = "failed";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventEnvelope {
    pub event_id: String,
    pub event_type: String,
    pub source: String,
    pub aggregate_id: String,
    pub occurred_at: DateTime<Utc>,
    pub payload: Value,
}

impl EventEnvelope {
    pub fn new(
        event_type: impl Into<String>,
        source: impl Into<String>,
        aggregate_id: impl Into<String>,
        payload: Value,
    ) -> Self {
        Self {
            event_id: Uuid::now_v7().to_string(),
            event_type: event_type.into(),
            source: source.into(),
            aggregate_id: aggregate_id.into(),
            occurred_at: Utc::now(),
            payload,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebateParticipantJoinedEvent {
    pub ws_id: u64,
    pub session_id: u64,
    pub user_id: u64,
    pub side: String,
    pub pro_count: i32,
    pub con_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebateSessionStatusChangedEvent {
    pub session_id: u64,
    pub from_status: String,
    pub to_status: String,
    pub changed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebateMessagePinnedEvent {
    pub ws_id: u64,
    pub session_id: u64,
    pub message_id: u64,
    pub user_id: u64,
    pub ledger_id: u64,
    pub cost_coins: i64,
    pub pin_seconds: i32,
    pub pinned_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiJudgeJobCreatedEvent {
    pub ws_id: u64,
    pub session_id: u64,
    pub job_id: u64,
    pub requested_by: u64,
    pub style_mode: String,
    pub rejudge_triggered: bool,
    pub requested_at: DateTime<Utc>,
}

#[derive(Clone)]
pub(crate) struct KafkaEventBus {
    producer: FutureProducer,
    config: KafkaConfig,
}

#[derive(Clone)]
pub(crate) enum EventBus {
    Disabled,
    Kafka(Arc<KafkaEventBus>),
}

impl EventBus {
    pub fn from_config(config: &KafkaConfig) -> anyhow::Result<Self> {
        if !config.enabled {
            return Ok(Self::Disabled);
        }

        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &config.brokers)
            .set("client.id", &config.client_id)
            .set("message.timeout.ms", config.producer_timeout_ms.to_string())
            .create()
            .context("create kafka producer failed")?;

        let bus = Self::Kafka(Arc::new(KafkaEventBus {
            producer,
            config: config.clone(),
        }));
        info!(
            "kafka event bus enabled, brokers={}, topic_prefix={}",
            config.brokers, config.topic_prefix
        );
        Ok(bus)
    }

    pub async fn publish_debate_participant_joined(
        &self,
        event: DebateParticipantJoinedEvent,
    ) -> anyhow::Result<()> {
        let aggregate_id = format!("session:{}", event.session_id);
        let payload = serde_json::to_value(event)?;
        let envelope = EventEnvelope::new(
            "debate.participant.joined",
            "chat-server",
            aggregate_id,
            payload,
        );
        let key = envelope.aggregate_id.clone();
        self.publish(TOPIC_DEBATE_PARTICIPANT_JOINED, &key, &envelope)
            .await
    }

    pub async fn publish_debate_session_status_changed(
        &self,
        event: DebateSessionStatusChangedEvent,
    ) -> anyhow::Result<()> {
        let aggregate_id = format!("session:{}", event.session_id);
        let payload = serde_json::to_value(event)?;
        let envelope = EventEnvelope::new(
            "debate.session.status.changed",
            "chat-server",
            aggregate_id,
            payload,
        );
        let key = envelope.aggregate_id.clone();
        self.publish(TOPIC_DEBATE_SESSION_STATUS_CHANGED, &key, &envelope)
            .await
    }

    pub async fn publish_debate_message_pinned(
        &self,
        event: DebateMessagePinnedEvent,
    ) -> anyhow::Result<()> {
        let aggregate_id = format!("session:{}", event.session_id);
        let payload = serde_json::to_value(event)?;
        let envelope = EventEnvelope::new(
            "debate.message.pinned",
            "chat-server",
            aggregate_id,
            payload,
        );
        let key = envelope.aggregate_id.clone();
        self.publish(TOPIC_DEBATE_MESSAGE_PINNED, &key, &envelope)
            .await
    }

    pub async fn publish_ai_judge_job_created(
        &self,
        event: AiJudgeJobCreatedEvent,
    ) -> anyhow::Result<()> {
        let aggregate_id = format!("session:{}", event.session_id);
        let payload = serde_json::to_value(event)?;
        let envelope =
            EventEnvelope::new("ai.judge.job.created", "chat-server", aggregate_id, payload);
        let key = envelope.aggregate_id.clone();
        self.publish(TOPIC_AI_JUDGE_JOB_CREATED, &key, &envelope)
            .await
    }

    pub async fn publish(
        &self,
        base_topic: &str,
        key: &str,
        event: &EventEnvelope,
    ) -> anyhow::Result<()> {
        match self {
            EventBus::Disabled => Ok(()),
            EventBus::Kafka(bus) => {
                let topic = bus.config.topic_name(base_topic);
                let payload = serde_json::to_string(event)?;
                bus.producer
                    .send(
                        FutureRecord::to(&topic).key(key).payload(&payload),
                        Timeout::After(Duration::from_millis(bus.config.producer_timeout_ms)),
                    )
                    .await
                    .map_err(|(e, _)| anyhow::anyhow!("kafka publish to {} failed: {}", topic, e))
                    .map(|delivery| {
                        debug!(
                            "kafka published topic={} partition={} offset={}",
                            topic, delivery.0, delivery.1
                        );
                    })
            }
        }
    }

    pub fn maybe_spawn_consumer_worker(&self, pool: PgPool) -> anyhow::Result<()> {
        let EventBus::Kafka(bus) = self else {
            return Ok(());
        };
        if !kafka_worker_enabled(&bus.config) {
            return Ok(());
        }

        let topics = if bus.config.consume_topics.is_empty() {
            vec![
                bus.config.topic_name(TOPIC_DEBATE_PARTICIPANT_JOINED),
                bus.config.topic_name(TOPIC_DEBATE_SESSION_STATUS_CHANGED),
                bus.config.topic_name(TOPIC_DEBATE_MESSAGE_PINNED),
                bus.config.topic_name(TOPIC_AI_JUDGE_JOB_CREATED),
            ]
        } else {
            bus.config
                .consume_topics
                .iter()
                .map(|topic| bus.config.topic_name(topic))
                .collect()
        };
        let worker_group_id = if bus.config.consumer.worker_group_id.trim().is_empty() {
            bus.config.group_id.clone()
        } else {
            bus.config.consumer.worker_group_id.clone()
        };

        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", &bus.config.brokers)
            .set("group.id", &worker_group_id)
            .set("client.id", format!("{}-consumer", bus.config.client_id))
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "false")
            .create()
            .context("create kafka consumer failed")?;
        let topic_refs: Vec<&str> = topics.iter().map(String::as_str).collect();
        consumer
            .subscribe(&topic_refs)
            .context("subscribe kafka topics failed")?;

        tokio::spawn(async move {
            info!(
                "kafka consumer worker started, group_id={}, topics={:?}",
                worker_group_id, topics
            );
            loop {
                match consumer.recv().await {
                    Err(e) => warn!("kafka consume failed: {}", e),
                    Ok(msg) => {
                        let topic = msg.topic().to_string();
                        let partition = msg.partition();
                        let offset = msg.offset();
                        let key = msg.key().map(|v| String::from_utf8_lossy(v).to_string());
                        let payload = match msg.payload_view::<str>() {
                            None => None,
                            Some(Ok(v)) => Some(v),
                            Some(Err(_)) => None,
                        };
                        let outcome = consume_worker_message(
                            &pool,
                            &worker_group_id,
                            &topic,
                            partition,
                            offset,
                            payload,
                        )
                        .await;
                        match outcome {
                            Ok(ret) => {
                                if let Err(err) = consumer.commit_message(&msg, CommitMode::Async) {
                                    warn!(
                                        "kafka commit failed topic={} partition={} offset={}: {}",
                                        topic, partition, offset, err
                                    );
                                } else {
                                    info!(
                                        "kafka worker consumed topic={} partition={} offset={} key={:?} outcome={}",
                                        topic, partition, offset, key, ret
                                    );
                                }
                            }
                            Err(err) => {
                                warn!(
                                    "kafka worker process failed topic={} partition={} offset={} key={:?}: {}",
                                    topic, partition, offset, key, err
                                );
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum ConsumeProcessOutcome {
    Succeeded,
    Failed,
    Duplicated,
}

impl std::fmt::Display for ConsumeProcessOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Succeeded => write!(f, "succeeded"),
            Self::Failed => write!(f, "failed"),
            Self::Duplicated => write!(f, "duplicated"),
        }
    }
}

#[derive(Debug)]
enum BusinessProcessOutcome {
    Succeeded,
    FailedPermanently(String),
}

fn kafka_worker_enabled(config: &KafkaConfig) -> bool {
    config.consume_enabled || config.consumer.worker_enabled
}

fn fallback_event_id(topic: &str, partition: i32, offset: i64) -> String {
    format!("{topic}:{partition}:{offset}")
}

async fn consume_worker_message(
    pool: &PgPool,
    consumer_group: &str,
    topic: &str,
    partition: i32,
    offset: i64,
    payload: Option<&str>,
) -> anyhow::Result<ConsumeProcessOutcome> {
    let payload_text = payload.unwrap_or_default();
    let envelope = match serde_json::from_str::<EventEnvelope>(payload_text) {
        Ok(v) => v,
        Err(err) => {
            let row = FailedLedgerRow {
                consumer_group: consumer_group.to_string(),
                topic: topic.to_string(),
                partition,
                offset,
                event_id: fallback_event_id(topic, partition, offset),
                event_type: "invalid.envelope".to_string(),
                aggregate_id: String::new(),
                payload: serde_json::json!({ "raw": payload_text }),
                error_message: format!("decode envelope failed: {err}"),
            };
            persist_failed_ledger_row(pool, &row).await?;
            return Ok(ConsumeProcessOutcome::Failed);
        }
    };

    process_business_event(pool, consumer_group, topic, partition, offset, &envelope).await
}

async fn process_business_event(
    pool: &PgPool,
    consumer_group: &str,
    topic: &str,
    partition: i32,
    offset: i64,
    envelope: &EventEnvelope,
) -> anyhow::Result<ConsumeProcessOutcome> {
    let mut tx = pool.begin().await?;
    let ledger_id = sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO kafka_consume_ledger(
            consumer_group, topic, partition, message_offset,
            event_id, event_type, aggregate_id, payload,
            status, error_message, processed_at, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NULL, NOW(), NOW(), NOW())
        ON CONFLICT (consumer_group, event_id) DO NOTHING
        RETURNING id
        "#,
    )
    .bind(consumer_group)
    .bind(topic)
    .bind(partition)
    .bind(offset)
    .bind(&envelope.event_id)
    .bind(&envelope.event_type)
    .bind(&envelope.aggregate_id)
    .bind(&envelope.payload)
    .bind(LEDGER_STATUS_SUCCEEDED)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(ledger_id) = ledger_id else {
        tx.rollback().await?;
        return Ok(ConsumeProcessOutcome::Duplicated);
    };

    match apply_worker_business_logic(&mut tx, envelope).await? {
        BusinessProcessOutcome::Succeeded => {
            tx.commit().await?;
            Ok(ConsumeProcessOutcome::Succeeded)
        }
        BusinessProcessOutcome::FailedPermanently(error_message) => {
            sqlx::query(
                r#"
                UPDATE kafka_consume_ledger
                SET status = $2,
                    error_message = $3,
                    processed_at = NOW(),
                    updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(ledger_id)
            .bind(LEDGER_STATUS_FAILED)
            .bind(error_message)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok(ConsumeProcessOutcome::Failed)
        }
    }
}

async fn apply_worker_business_logic(
    tx: &mut Transaction<'_, Postgres>,
    envelope: &EventEnvelope,
) -> anyhow::Result<BusinessProcessOutcome> {
    if envelope.event_type == "ai.judge.job.created" {
        let payload: AiJudgeJobCreatedEvent = match serde_json::from_value(envelope.payload.clone())
        {
            Ok(v) => v,
            Err(err) => {
                return Ok(BusinessProcessOutcome::FailedPermanently(format!(
                    "decode ai.judge.job.created payload failed: {err}"
                )))
            }
        };
        sqlx::query(
            r#"
            UPDATE judge_jobs
            SET dispatch_locked_until = NOW() - INTERVAL '1 second',
                updated_at = NOW()
            WHERE id = $1
              AND ws_id = $2
              AND session_id = $3
              AND status = 'running'
            "#,
        )
        .bind(payload.job_id as i64)
        .bind(payload.ws_id as i64)
        .bind(payload.session_id as i64)
        .execute(&mut **tx)
        .await?;
    }
    Ok(BusinessProcessOutcome::Succeeded)
}

struct FailedLedgerRow {
    consumer_group: String,
    topic: String,
    partition: i32,
    offset: i64,
    event_id: String,
    event_type: String,
    aggregate_id: String,
    payload: Value,
    error_message: String,
}

async fn persist_failed_ledger_row(pool: &PgPool, row: &FailedLedgerRow) -> anyhow::Result<()> {
    let _ = sqlx::query(
        r#"
        INSERT INTO kafka_consume_ledger(
            consumer_group, topic, partition, message_offset,
            event_id, event_type, aggregate_id, payload,
            status, error_message, processed_at, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW(), NOW(), NOW())
        ON CONFLICT (consumer_group, event_id)
        DO UPDATE SET
            status = EXCLUDED.status,
            error_message = EXCLUDED.error_message,
            processed_at = NOW(),
            updated_at = NOW()
        "#,
    )
    .bind(&row.consumer_group)
    .bind(&row.topic)
    .bind(row.partition)
    .bind(row.offset)
    .bind(&row.event_id)
    .bind(&row.event_type)
    .bind(&row.aggregate_id)
    .bind(&row.payload)
    .bind(LEDGER_STATUS_FAILED)
    .bind(&row.error_message)
    .execute(pool)
    .await?;
    Ok(())
}

impl KafkaConfig {
    pub fn topic_name(&self, base_topic: &str) -> String {
        let prefix = self.topic_prefix.trim();
        if prefix.is_empty() {
            base_topic.to_string()
        } else {
            format!("{}.{}", prefix, base_topic)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kafka_topic_name_should_apply_prefix() {
        let cfg = KafkaConfig {
            topic_prefix: "aicomm".to_string(),
            ..Default::default()
        };
        assert_eq!(
            cfg.topic_name(TOPIC_DEBATE_PARTICIPANT_JOINED),
            "aicomm.debate.participant.joined.v1"
        );
        assert_eq!(
            cfg.topic_name(TOPIC_DEBATE_SESSION_STATUS_CHANGED),
            "aicomm.debate.session.status.changed.v1"
        );
        assert_eq!(
            cfg.topic_name(TOPIC_DEBATE_MESSAGE_PINNED),
            "aicomm.debate.message.pinned.v1"
        );
        assert_eq!(
            cfg.topic_name(TOPIC_AI_JUDGE_JOB_CREATED),
            "aicomm.ai.judge.job.created.v1"
        );
    }

    #[test]
    fn event_envelope_new_should_fill_required_fields() {
        let event = EventEnvelope::new(
            "debate.participant.joined",
            "chat-server",
            "session:42",
            serde_json::json!({"sessionId": 42}),
        );
        assert!(!event.event_id.is_empty());
        assert_eq!(event.event_type, "debate.participant.joined");
        assert_eq!(event.source, "chat-server");
        assert_eq!(event.aggregate_id, "session:42");
        assert_eq!(event.payload["sessionId"], 42);
    }

    #[test]
    fn fallback_event_id_should_include_topic_partition_offset() {
        let id = fallback_event_id("aicomm.ai.judge.job.created.v1", 2, 18);
        assert_eq!(id, "aicomm.ai.judge.job.created.v1:2:18");
    }

    #[test]
    fn kafka_worker_enabled_should_allow_legacy_or_worker_switch() {
        let cfg = KafkaConfig {
            consume_enabled: true,
            ..Default::default()
        };
        assert!(kafka_worker_enabled(&cfg));

        let cfg = KafkaConfig {
            consumer: crate::config::KafkaConsumerConfig {
                worker_enabled: true,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(kafka_worker_enabled(&cfg));

        let cfg = KafkaConfig::default();
        assert!(!kafka_worker_enabled(&cfg));
    }
}
