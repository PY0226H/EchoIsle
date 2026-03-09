CREATE TABLE IF NOT EXISTS ops_service_split_reviews(
  ws_id bigint PRIMARY KEY REFERENCES workspaces(id) ON DELETE CASCADE,
  payment_compliance_required boolean,
  review_note text NOT NULL DEFAULT '',
  updated_by bigint NOT NULL REFERENCES users(id),
  created_at timestamptz NOT NULL DEFAULT NOW(),
  updated_at timestamptz NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ops_service_split_reviews_updated_at
  ON ops_service_split_reviews(updated_at DESC);
