import { describe, expect, it } from 'vitest';
import { requireAuthSecret, requireAuthUrl } from './auth-secret';

describe('requireAuthSecret', () => {
	it('returns a configured secret', () => {
		expect(requireAuthSecret('a-long-enough-secret-value')).toBe('a-long-enough-secret-value');
	});

	it('throws when the secret is missing', () => {
		expect(() => requireAuthSecret(undefined)).toThrow('BETTER_AUTH_SECRET is not configured');
	});
});

describe('requireAuthUrl', () => {
	it('returns a configured URL', () => {
		expect(requireAuthUrl('https://eisen.example')).toBe('https://eisen.example');
	});

	it('throws when the URL is missing', () => {
		expect(() => requireAuthUrl(undefined)).toThrow('BETTER_AUTH_URL is not configured');
	});
});
