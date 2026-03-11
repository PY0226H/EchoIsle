-- single-tenant compatibility shim after workspace concept removal.
-- keep ws_id as internal constant scope key (1) so legacy SQL paths remain valid.

ALTER TABLE users
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;

ALTER TABLE chats
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;

ALTER TABLE debate_topics
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;

ALTER TABLE debate_sessions
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;

ALTER TABLE session_messages
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;

ALTER TABLE session_pinned_messages
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;

ALTER TABLE judge_jobs
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;

ALTER TABLE judge_stage_summaries
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;

ALTER TABLE judge_reports
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;

ALTER TABLE judge_draw_votes
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;

ALTER TABLE judge_draw_vote_ballots
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;

ALTER TABLE iap_orders
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;

ALTER TABLE user_wallets
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;

ALTER TABLE wallet_ledger
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;

ALTER TABLE kafka_dlq_events
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;

ALTER TABLE auth_refresh_sessions
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;

ALTER TABLE ops_alert_notifications
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;

ALTER TABLE ops_alert_states
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;

ALTER TABLE ops_observability_configs
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;

ALTER TABLE ops_service_split_reviews
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;

ALTER TABLE ops_service_split_review_audits
  ADD COLUMN IF NOT EXISTS ws_id bigint NOT NULL DEFAULT 1;
