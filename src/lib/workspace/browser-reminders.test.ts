import { describe, expect, it } from 'vitest';
import { browserReminders } from './browser-reminders';

describe('browserReminders', () => {
	it('reports vapidReady from the runtime public key', () => {
		expect(browserReminders('Babc').vapidReady()).toBe(true);
		expect(browserReminders('').vapidReady()).toBe(false);
	});
});
