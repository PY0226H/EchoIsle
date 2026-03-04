CREATE TABLE IF NOT EXISTS ops_observability_configs(
  ws_id bigint PRIMARY KEY REFERENCES workspaces(id) ON DELETE CASCADE,
  thresholds_json jsonb NOT NULL DEFAULT '{}'::jsonb,
  anomaly_state_json jsonb NOT NULL DEFAULT '{}'::jsonb,
  updated_by bigint NOT NULL REFERENCES users(id),
  created_at timestamptz NOT NULL DEFAULT NOW(),
  updated_at timestamptz NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ops_observability_configs_updated_at
  ON ops_observability_configs(updated_at DESC);
