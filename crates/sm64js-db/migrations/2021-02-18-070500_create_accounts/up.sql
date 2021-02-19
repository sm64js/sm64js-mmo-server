CREATE TABLE accounts (
  id SERIAL PRIMARY KEY,
  username VARCHAR,
  discord_username VARCHAR,
  discord_discriminator VARCHAR,
  google_sub VARCHAR REFERENCES google_accounts ON DELETE RESTRICT,
  FOREIGN KEY (discord_username, discord_discriminator) REFERENCES discord_accounts ON DELETE RESTRICT
)
