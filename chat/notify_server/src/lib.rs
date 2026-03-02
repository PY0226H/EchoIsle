mod config;
mod error;
mod middlewares;
mod notif;
mod sse;
mod ws;

use axum::{
    http::Method,
    middleware::from_fn_with_state,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use chat_core::{middlewares::TokenVerify, DecodingKey, User};
use dashmap::DashMap;
use middlewares::verify_notify_ticket;
use serde_json::Value;
use sse::sse_handler;
use std::{
    collections::VecDeque,
    ops::Deref,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::broadcast;
use tower_http::cors::{self, CorsLayer};
use ws::{debate_room_ws_handler, ws_handler};

pub use config::AppConfig;
pub use error::AppError;
pub use notif::AppEvent;

const CHANNEL_CAPACITY: usize = 256;
const DEBATE_REPLAY_HISTORY_CAPACITY: usize = 400;
const DEBATE_REPLAY_MAX_ON_CONNECT: usize = 200;

pub type UserMap = Arc<DashMap<u64, broadcast::Sender<Arc<UserEvent>>>>;
type DebateReplayMap = Arc<DashMap<(u64, i64), DebateReplayHistory>>;

#[derive(Debug, Clone)]
pub struct DebateReplayEvent {
    pub session_id: i64,
    pub event_seq: u64,
    pub event_name: String,
    pub payload: Value,
    pub event_at_ms: i64,
}

#[derive(Debug, Clone)]
pub struct UserEvent {
    pub app_event: Arc<AppEvent>,
    pub debate_replay: Option<DebateReplayEvent>,
}

#[derive(Debug)]
struct DebateReplayHistory {
    next_seq: u64,
    events: VecDeque<DebateReplayEvent>,
}

#[derive(Debug, Clone, Default)]
pub struct DebateReplayWindow {
    pub events: Vec<DebateReplayEvent>,
    pub latest_seq: u64,
    pub has_gap: bool,
    pub skipped: u64,
}

#[derive(Clone)]
pub struct AppState(Arc<AppStateInner>);

pub struct AppStateInner {
    pub config: AppConfig,
    users: UserMap,
    debate_replays: DebateReplayMap,
    dk: DecodingKey,
}

const INDEX_HTML: &str = include_str!("../index.html");

pub async fn get_router(config: AppConfig) -> anyhow::Result<Router> {
    let state = AppState::new(config);
    notif::setup_pg_listener(state.clone()).await?;

    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::PUT,
        ])
        .allow_origin(cors::Any)
        .allow_headers(cors::Any);

    let app = Router::new()
        .route("/events", get(sse_handler))
        .route("/ws", get(ws_handler))
        .route("/ws/debate/:session_id", get(debate_room_ws_handler))
        .layer(from_fn_with_state(state.clone(), verify_notify_ticket))
        .layer(cors)
        .route("/", get(index_handler))
        .route("/health", get(health_handler))
        .with_state(state);

    Ok(app)
}

async fn index_handler() -> impl IntoResponse {
    Html(INDEX_HTML)
}

async fn health_handler() -> &'static str {
    "ok"
}

impl TokenVerify for AppState {
    type Error = AppError;

    fn verify(&self, token: &str) -> Result<User, Self::Error> {
        Ok(self.dk.verify(token)?)
    }
}

impl Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let dk = DecodingKey::load(&config.auth.pk).expect("Failed to load public key");
        let users = Arc::new(DashMap::new());
        let debate_replays = Arc::new(DashMap::new());
        Self(Arc::new(AppStateInner {
            config,
            dk,
            users,
            debate_replays,
        }))
    }

    pub(crate) fn subscribe_user_events(
        &self,
        user_id: u64,
    ) -> broadcast::Receiver<Arc<UserEvent>> {
        if let Some(tx) = self.users.get(&user_id) {
            tx.subscribe()
        } else {
            let (tx, rx) = broadcast::channel(CHANNEL_CAPACITY);
            self.users.insert(user_id, tx);
            rx
        }
    }

    pub(crate) fn cleanup_user_events_if_unused(&self, user_id: u64) {
        let should_remove = self
            .users
            .get(&user_id)
            .map(|tx| tx.receiver_count() == 0)
            .unwrap_or(false);
        if should_remove {
            self.users.remove(&user_id);
        }
    }

    pub(crate) fn build_user_event_for_recipient(
        &self,
        user_id: u64,
        app_event: Arc<AppEvent>,
    ) -> UserEvent {
        let debate_replay = self.append_debate_replay_event(user_id, app_event.as_ref());
        UserEvent {
            app_event,
            debate_replay,
        }
    }

    pub(crate) fn replay_debate_events_for_user(
        &self,
        user_id: u64,
        session_id: i64,
        last_ack_seq: Option<u64>,
    ) -> DebateReplayWindow {
        let from_seq = last_ack_seq.unwrap_or(0);
        let Some(history) = self.debate_replays.get(&(user_id, session_id)) else {
            return DebateReplayWindow {
                events: vec![],
                latest_seq: 0,
                has_gap: false,
                skipped: 0,
            };
        };
        let latest_seq = history.latest_seq();
        if history.events.is_empty() || from_seq >= latest_seq {
            return DebateReplayWindow {
                events: vec![],
                latest_seq,
                has_gap: false,
                skipped: 0,
            };
        }

        let first_seq = history
            .events
            .front()
            .map(|evt| evt.event_seq)
            .unwrap_or(latest_seq + 1);
        let has_gap = from_seq.saturating_add(1) < first_seq;
        let skipped = if has_gap {
            first_seq.saturating_sub(from_seq.saturating_add(1))
        } else {
            0
        };
        let events = history
            .events
            .iter()
            .filter(|evt| evt.event_seq > from_seq)
            .take(DEBATE_REPLAY_MAX_ON_CONNECT)
            .cloned()
            .collect::<Vec<_>>();

        DebateReplayWindow {
            events,
            latest_seq,
            has_gap,
            skipped,
        }
    }

    fn append_debate_replay_event(
        &self,
        user_id: u64,
        app_event: &AppEvent,
    ) -> Option<DebateReplayEvent> {
        let session_id = app_event.debate_session_id()?;
        let payload = serde_json::to_value(app_event).ok()?;
        let raw = DebateReplayEvent {
            session_id,
            event_seq: 0,
            event_name: app_event.event_name().to_string(),
            payload,
            event_at_ms: now_unix_ms(),
        };

        let mut entry = self
            .debate_replays
            .entry((user_id, session_id))
            .or_default();
        Some(entry.push(raw))
    }
}

impl DebateReplayHistory {
    fn latest_seq(&self) -> u64 {
        self.next_seq.saturating_sub(1)
    }

    fn push(&mut self, mut event: DebateReplayEvent) -> DebateReplayEvent {
        event.event_seq = self.next_seq;
        self.next_seq = self.next_seq.saturating_add(1);
        self.events.push_back(event.clone());
        while self.events.len() > DEBATE_REPLAY_HISTORY_CAPACITY {
            let _ = self.events.pop_front();
        }
        event
    }
}

impl Default for DebateReplayHistory {
    fn default() -> Self {
        Self {
            next_seq: 1,
            events: VecDeque::new(),
        }
    }
}

impl UserEvent {
    pub fn debate_session_id(&self) -> Option<i64> {
        self.debate_replay
            .as_ref()
            .map(|v| v.session_id)
            .or_else(|| self.app_event.debate_session_id())
    }
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_handler_should_return_ok() {
        assert_eq!(health_handler().await, "ok");
    }
}
