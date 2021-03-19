UPDATE accounts
  SET last_ip = NULL
  WHERE last_ip = '0.0.0.0';
