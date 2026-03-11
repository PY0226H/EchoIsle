ALTER TABLE users
  ALTER COLUMN email DROP NOT NULL;

ALTER TABLE users
  ADD COLUMN IF NOT EXISTS phone_e164 varchar(20),
  ADD COLUMN IF NOT EXISTS phone_verified_at timestamptz,
  ADD COLUMN IF NOT EXISTS phone_bind_required boolean NOT NULL DEFAULT true;

CREATE UNIQUE INDEX IF NOT EXISTS users_phone_e164_unique_idx
  ON users(phone_e164)
  WHERE phone_e164 IS NOT NULL;

UPDATE users
SET phone_bind_required = true
WHERE id <> 0 AND phone_e164 IS NULL;
