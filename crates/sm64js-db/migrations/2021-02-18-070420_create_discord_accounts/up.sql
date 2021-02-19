CREATE TABLE discord_accounts (
  username VARCHAR NOT NULL,
  discriminator VARCHAR NOT NULL,
  avatar VARCHAR,
  mfa_enabled BOOLEAN,
  locale VARCHAR,
  flags INTEGER,
  premium_type SMALLINT,
  public_flags INTEGER,
  session INTEGER REFERENCES discord_sessions ON DELETE SET NULL,
  PRIMARY KEY (username, discriminator)
)
