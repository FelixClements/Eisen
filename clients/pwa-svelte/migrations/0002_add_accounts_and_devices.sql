CREATE TABLE IF NOT EXISTS accounts (
  owner_id TEXT PRIMARY KEY,
  vault_id TEXT NOT NULL UNIQUE,
  created_at INTEGER NOT NULL,
  last_sync_at INTEGER,
  device_count INTEGER DEFAULT 1
);

CREATE TABLE IF NOT EXISTS devices (
  device_id TEXT PRIMARY KEY,
  owner_id TEXT NOT NULL,
  enrolled_at INTEGER NOT NULL,
  last_seen_at INTEGER,
  revoked_at INTEGER,
  FOREIGN KEY (owner_id) REFERENCES accounts(owner_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_devices_owner ON devices(owner_id);

CREATE TABLE IF NOT EXISTS backups (
  package_id TEXT PRIMARY KEY,
  owner_id TEXT NOT NULL,
  r2_key TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  FOREIGN KEY (owner_id) REFERENCES accounts(owner_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_backups_owner ON backups(owner_id);
