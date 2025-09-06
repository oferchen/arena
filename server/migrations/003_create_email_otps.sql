CREATE TABLE IF NOT EXISTS email_otps (
  id BIGSERIAL PRIMARY KEY,
  user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  otp_code TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMPTZ NOT NULL,
  UNIQUE (user_id, otp_code)
);

CREATE INDEX IF NOT EXISTS idx_email_otps_user_id
  ON email_otps(user_id);
