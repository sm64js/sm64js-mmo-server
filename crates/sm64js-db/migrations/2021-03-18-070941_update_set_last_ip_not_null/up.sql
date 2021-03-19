UPDATE accounts
  SET last_ip = '0.0.0.0'
  WHERE last_ip IS NULL;
