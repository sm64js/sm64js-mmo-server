CREATE TABLE ip_bans (
  ip VARCHAR PRIMARY KEY,
  reason VARCHAR,
  expires_at TIMESTAMP
)
