-- judge draw vote domain for AI draw-resolution workflow

CREATE TABLE IF NOT EXISTS judge_draw_votes(
  id bigserial PRIMARY KEY,
  ws_id bigint NOT NULL REFERENCES workspaces(id),
  session_id bigint NOT NULL REFERENCES debate_sessions(id) ON DELETE CASCADE,
  report_id bigint NOT NULL UNIQUE REFERENCES judge_reports(id) ON DELETE CASCADE,
  threshold_percent int NOT NULL DEFAULT 70 CHECK (threshold_percent > 0 AND threshold_percent <= 100),
  eligible_voters int NOT NULL CHECK (eligible_voters >= 0),
  required_voters int NOT NULL CHECK (required_voters >= 0),
  voting_ends_at timestamptz NOT NULL,
  status varchar(16) NOT NULL CHECK (status IN ('open', 'decided', 'expired')),
  resolution varchar(20) NOT NULL CHECK (resolution IN ('pending', 'accept_draw', 'open_rematch')),
  decided_at timestamptz,
  created_at timestamptz DEFAULT CURRENT_TIMESTAMP,
  updated_at timestamptz DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_judge_draw_votes_ws_session
  ON judge_draw_votes(ws_id, session_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_judge_draw_votes_status_ends_at
  ON judge_draw_votes(status, voting_ends_at);

CREATE TABLE IF NOT EXISTS judge_draw_vote_ballots(
  id bigserial PRIMARY KEY,
  vote_id bigint NOT NULL REFERENCES judge_draw_votes(id) ON DELETE CASCADE,
  ws_id bigint NOT NULL REFERENCES workspaces(id),
  session_id bigint NOT NULL REFERENCES debate_sessions(id) ON DELETE CASCADE,
  report_id bigint NOT NULL REFERENCES judge_reports(id) ON DELETE CASCADE,
  user_id bigint NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  agree_draw boolean NOT NULL,
  voted_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(vote_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_judge_draw_vote_ballots_vote
  ON judge_draw_vote_ballots(vote_id, voted_at DESC);
CREATE INDEX IF NOT EXISTS idx_judge_draw_vote_ballots_session_user
  ON judge_draw_vote_ballots(session_id, user_id);
