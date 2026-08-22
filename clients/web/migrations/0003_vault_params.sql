CREATE TABLE IF NOT EXISTS vault_params (
	user_id TEXT PRIMARY KEY NOT NULL,
	kdf_salt TEXT NOT NULL,
	check_blob TEXT NOT NULL,
	created_at INTEGER NOT NULL
);
