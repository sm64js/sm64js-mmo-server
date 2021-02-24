CREATE TABLE google_sessions (
  id SERIAL PRIMARY KEY,
  id_token VARCHAR NOT NULL,
  expires_at TIMESTAMP NOT NULL,
  google_account_id VARCHAR NOT NULL REFERENCES google_accounts ON DELETE CASCADE
)
