ALTER TABLE push_subscriptions ADD COLUMN device_id TEXT;

CREATE INDEX IF NOT EXISTS idx_push_subscriptions_user_device ON push_subscriptions(user_id, device_id);

DELETE FROM push_subscriptions WHERE device_id IS NULL;
