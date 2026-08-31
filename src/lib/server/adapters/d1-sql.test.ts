import { describe, expect, it } from 'vitest';
import { UPSERT_BACKUP_SQL, UPSERT_VAULT_RECORD_SQL } from './d1';

describe('D1 upsert isolation SQL', () => {
	it('vault_records conflicts on (user_id, record_id) and does not reassign user_id', () => {
		expect(UPSERT_VAULT_RECORD_SQL).toMatch(/ON CONFLICT\(user_id, record_id\)/);
		expect(UPSERT_VAULT_RECORD_SQL).not.toMatch(/user_id\s*=\s*excluded\.user_id/);
	});

	it('backups conflicts on (user_id, package_id) and does not reassign user_id', () => {
		expect(UPSERT_BACKUP_SQL).toMatch(/ON CONFLICT\(user_id, package_id\)/);
		expect(UPSERT_BACKUP_SQL).not.toMatch(/user_id\s*=\s*excluded\.user_id/);
	});
});
