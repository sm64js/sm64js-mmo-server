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
  account_id INTEGER NOT NULL REFERENCES accounts ON DELETE CASCADE
)
