CREATE TABLE IF NOT EXISTS ops_service_split_review_audits(
  id bigserial PRIMARY KEY,
  ws_id bigint NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  payment_compliance_required boolean,
  review_note text NOT NULL DEFAULT '',
  updated_by bigint NOT NULL REFERENCES users(id),
  created_at timestamptz NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ops_service_split_review_audits_ws_created_at
  ON ops_service_split_review_audits(ws_id, created_at DESC, id DESC);
