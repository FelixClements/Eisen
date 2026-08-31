-- Composite primary keys so one Account cannot steal another Account's row
-- by colliding on a client UUID (task record_id or recovery package_id).

CREATE TABLE vault_records_new (
	user_id TEXT NOT NULL,
	record_id TEXT NOT NULL,
	encrypted_blob BLOB NOT NULL,
	modified_at INTEGER NOT NULL,
	device_id TEXT NOT NULL DEFAULT '',
	sync_version INTEGER NOT NULL,
	deleted INTEGER NOT NULL DEFAULT 0,
	PRIMARY KEY (user_id, record_id)
);

INSERT INTO vault_records_new (user_id, record_id, encrypted_blob, modified_at, device_id, sync_version, deleted)
SELECT user_id, record_id, encrypted_blob, modified_at, device_id, sync_version, deleted
FROM vault_records;

DROP TABLE vault_records;
ALTER TABLE vault_records_new RENAME TO vault_records;

CREATE INDEX idx_vault_records_user_version ON vault_records(user_id, sync_version);

CREATE TABLE backups_new (
	user_id TEXT NOT NULL,
	package_id TEXT NOT NULL,
	r2_key TEXT NOT NULL,
	created_at INTEGER NOT NULL,
	PRIMARY KEY (user_id, package_id)
);

INSERT INTO backups_new (user_id, package_id, r2_key, created_at)
SELECT user_id, package_id, r2_key, created_at
FROM backups;

DROP TABLE backups;
ALTER TABLE backups_new RENAME TO backups;

CREATE INDEX idx_backups_user ON backups(user_id);
