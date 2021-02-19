CREATE TABLE google_accounts (
  sub VARCHAR PRIMARY KEY,
  session INTEGER REFERENCES google_sessions ON DELETE SET NULL
)
