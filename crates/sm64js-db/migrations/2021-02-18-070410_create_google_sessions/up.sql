CREATE TABLE google_sessions (
  id SERIAL PRIMARY KEY,
  id_token VARCHAR NOT NULL,
  expires_in BIGINT NOT NULL
)
