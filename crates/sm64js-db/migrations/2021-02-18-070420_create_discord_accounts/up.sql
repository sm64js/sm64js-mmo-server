CREATE TABLE discord_accounts (
  id VARCHAR PRIMARY KEY,
  username VARCHAR NOT NULL,
  discriminator VARCHAR NOT NULL,
  avatar VARCHAR,
  mfa_enabled BOOLEAN,
  locale VARCHAR,
  flags INTEGER,
  premium_type SMALLINT,
  public_flags INTEGER,
  nick VARCHAR,
  roles TEXT[] NOT NULL,
  joined_at VARCHAR NOT NULL,
  premium_since VARCHAR,
  deaf BOOLEAN NOT NULL,
  mute BOOLEAN NOT NULL,
  account_id INTEGER NOT NULL REFERENCES accounts ON DELETE CASCADE
)
