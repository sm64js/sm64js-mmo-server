CREATE TABLE discord_sessions (
  id SERIAL PRIMARY KEY,
  access_token VARCHAR NOT NULL,
  token_type VARCHAR NOT NULL,
  expires_at TIMESTAMP NOT NULL,
  discord_account_id VARCHAR NOT NULL REFERENCES discord_accounts ON DELETE CASCADE
)
