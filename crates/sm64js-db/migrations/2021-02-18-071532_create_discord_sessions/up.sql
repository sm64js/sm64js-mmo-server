CREATE TABLE discord_sessions (
  id SERIAL PRIMARY KEY,
  access_token VARCHAR NOT NULL,
  token_type VARCHAR NOT NULL,
  expires_in BIGINT NOT NULL
)
