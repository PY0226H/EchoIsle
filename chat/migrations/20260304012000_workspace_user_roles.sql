CREATE TABLE IF NOT EXISTS workspace_user_roles(
  ws_id bigint NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  user_id bigint NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  role varchar(32) NOT NULL CHECK (role IN ('ops_admin', 'ops_reviewer', 'ops_viewer')),
  granted_by bigint NOT NULL REFERENCES users(id),
  created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (ws_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_workspace_user_roles_ws_role
  ON workspace_user_roles(ws_id, role);

CREATE INDEX IF NOT EXISTS idx_workspace_user_roles_ws_user
  ON workspace_user_roles(ws_id, user_id);
