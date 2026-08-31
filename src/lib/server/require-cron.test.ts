import { error } from '@sveltejs/kit';
import { describe, expect, it } from 'vitest';
import { requireCronSecret } from './require-cron';

function eventWithAuth(auth?: string, secret = 'test-secret') {
	return {
		request: {
			headers: {
				get(name: string) {
					if (name.toLowerCase() === 'authorization') return auth ?? null;
					return null;
				}
			}
		},
		platform: { env: { CRON_SECRET: secret } }
	} as Parameters<typeof requireCronSecret>[0];
}

function expectUnauthorized(fn: () => void) {
	try {
		fn();
		throw new Error('expected unauthorized');
	} catch (err) {
		expect(err).toMatchObject({ status: 401 });
	}
}

describe('requireCronSecret', () => {
	it('accepts a valid bearer token', () => {
		expect(() => requireCronSecret(eventWithAuth('Bearer test-secret'))).not.toThrow();
	});

	it('rejects a missing authorization header', () => {
		expectUnauthorized(() => requireCronSecret(eventWithAuth()));
	});

	it('rejects a wrong bearer token', () => {
		expectUnauthorized(() => requireCronSecret(eventWithAuth('Bearer wrong')));
	});

	it('rejects a shorter token without throwing on length mismatch', () => {
		expectUnauthorized(() => requireCronSecret(eventWithAuth('Bearer x')));
	});
});
