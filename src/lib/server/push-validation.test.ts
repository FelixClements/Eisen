import { describe, expect, it } from 'vitest';
import {
	assertDeviceId,
	assertPushEndpoint,
	assertPushKeys,
	assertWakeAt
} from './push-validation';

describe('push-validation', () => {
	it('accepts valid device ids and wake times', () => {
		expect(() => assertDeviceId('00000000-0000-4000-8000-000000000001')).not.toThrow();
		const now = 1_000_000;
		expect(() => assertWakeAt(now + 60_000, now)).not.toThrow();
	});

	it('rejects invalid wake times', () => {
		const now = 1_000_000;
		expect(() => assertWakeAt(now - 1, now)).toThrow();
		expect(() => assertWakeAt(now + 400 * 24 * 60 * 60 * 1000, now)).toThrow();
	});

	it('validates push endpoint and keys', () => {
		expect(() => assertPushEndpoint('https://fcm.googleapis.com/fcm/send/x')).not.toThrow();
		expect(() => assertPushEndpoint('http://insecure.example')).toThrow();
		expect(() => assertPushEndpoint('https://evil.example/hook')).toThrow();
		assertPushKeys('A'.repeat(87), 'B'.repeat(22));
	});
});
