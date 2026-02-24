-- ai judge dispatch queue metadata

ALTER TABLE judge_jobs
  ADD COLUMN IF NOT EXISTS dispatch_attempts int NOT NULL DEFAULT 0 CHECK (dispatch_attempts >= 0);

ALTER TABLE judge_jobs
  ADD COLUMN IF NOT EXISTS last_dispatch_at timestamptz;

ALTER TABLE judge_jobs
  ADD COLUMN IF NOT EXISTS dispatch_locked_until timestamptz;

CREATE INDEX IF NOT EXISTS idx_judge_jobs_dispatch_due
  ON judge_jobs(status, dispatch_locked_until, requested_at);
