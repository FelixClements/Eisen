-- Better Auth core tables (SQLite / D1)
CREATE TABLE IF NOT EXISTS user (
	id TEXT PRIMARY KEY NOT NULL,
	name TEXT NOT NULL,
	email TEXT NOT NULL UNIQUE,
	emailVerified INTEGER NOT NULL DEFAULT 0,
	image TEXT,
	createdAt INTEGER NOT NULL,
	updatedAt INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS session (
	id TEXT PRIMARY KEY NOT NULL,
	expiresAt INTEGER NOT NULL,
	token TEXT NOT NULL UNIQUE,
	createdAt INTEGER NOT NULL,
	updatedAt INTEGER NOT NULL,
	ipAddress TEXT,
	userAgent TEXT,
	userId TEXT NOT NULL REFERENCES user(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS session_userId_idx ON session(userId);

CREATE TABLE IF NOT EXISTS account (
	id TEXT PRIMARY KEY NOT NULL,
	accountId TEXT NOT NULL,
	providerId TEXT NOT NULL,
	userId TEXT NOT NULL REFERENCES user(id) ON DELETE CASCADE,
	accessToken TEXT,
	refreshToken TEXT,
	idToken TEXT,
	accessTokenExpiresAt INTEGER,
	refreshTokenExpiresAt INTEGER,
	scope TEXT,
	password TEXT,
	issuer TEXT NOT NULL,
	createdAt INTEGER NOT NULL,
	updatedAt INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS account_userId_idx ON account(userId);
CREATE UNIQUE INDEX IF NOT EXISTS account_issuer_accountId_uidx ON account(issuer, accountId);

CREATE TABLE IF NOT EXISTS verification (
	id TEXT PRIMARY KEY NOT NULL,
	identifier TEXT NOT NULL,
	value TEXT NOT NULL,
	expiresAt INTEGER NOT NULL,
	createdAt INTEGER,
	updatedAt INTEGER
);

-- Eisen encrypted sync
CREATE TABLE IF NOT EXISTS vault_records (
	record_id TEXT PRIMARY KEY,
	user_id TEXT NOT NULL,
	encrypted_blob BLOB NOT NULL,
	modified_at INTEGER NOT NULL,
	sync_version INTEGER NOT NULL,
	deleted INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_vault_records_user_version ON vault_records(user_id, sync_version);

CREATE TABLE IF NOT EXISTS backups (
	package_id TEXT PRIMARY KEY,
	user_id TEXT NOT NULL,
	r2_key TEXT NOT NULL,
	created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_backups_user ON backups(user_id);

CREATE TABLE IF NOT EXISTS push_subscriptions (
	id TEXT PRIMARY KEY,
	user_id TEXT NOT NULL,
	endpoint TEXT NOT NULL UNIQUE,
	p256dh TEXT NOT NULL,
	auth TEXT NOT NULL,
	created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_push_subscriptions_user ON push_subscriptions(user_id);

CREATE TABLE IF NOT EXISTS wake_schedules (
	id TEXT PRIMARY KEY,
	user_id TEXT NOT NULL,
	device_id TEXT NOT NULL,
	wake_at INTEGER NOT NULL,
	nonce TEXT NOT NULL,
	sent INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_wake_schedules_pending ON wake_schedules(sent, wake_at);
