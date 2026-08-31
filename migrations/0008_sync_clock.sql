CREATE TABLE vault_sync_clock (
	user_id TEXT PRIMARY KEY,
	next_version INTEGER NOT NULL
);

INSERT INTO vault_sync_clock (user_id, next_version)
SELECT user_id, IFNULL(MAX(sync_version), 0) FROM vault_records GROUP BY user_id;
