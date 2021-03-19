CREATE TABLE geolocations (
  query VARCHAR PRIMARY KEY,
  country_code VARCHAR NOT NULL,
  region VARCHAR NOT NULL,
  city VARCHAR NOT NULL,
  zip VARCHAR NOT NULL,
  lat FLOAT NOT NULL,
  lon FLOAT NOT NULL,
  timezone VARCHAR NOT NULL,
  isp VARCHAR NOT NULL,
  mobile BOOLEAN NOT NULL,
  proxy	BOOLEAN NOT NULL,
  discord_session_id INTEGER REFERENCES discord_sessions ON DELETE SET NULL,
  google_session_id INTEGER REFERENCES google_sessions ON DELETE SET NULL,
  ban_id VARCHAR REFERENCES bans ON DELETE SET NULL
)
