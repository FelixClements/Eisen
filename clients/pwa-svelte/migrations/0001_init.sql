CREATE TABLE IF NOT EXISTS vault_records (
	record_id TEXT PRIMARY KEY,
	owner_id TEXT NOT NULL,
	encrypted_blob BLOB NOT NULL,
	modified_at INTEGER NOT NULL,
	sync_version INTEGER NOT NULL,
	deleted INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_vault_records_owner_version
ON vault_records(owner_id, sync_version);
