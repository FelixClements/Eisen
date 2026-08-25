import { describe, expect, it } from 'vitest';
import { applyRecordLww, remoteWins } from './record-lww';

describe('record-lww', () => {
	it('remoteWins keeps the newer modifiedAt', () => {
		expect(remoteWins({ modifiedAt: 200, deviceId: 'a' }, { modifiedAt: 100, deviceId: 'z' })).toBe(
			false
		);
		expect(remoteWins({ modifiedAt: 100, deviceId: 'z' }, { modifiedAt: 200, deviceId: 'a' })).toBe(
			true
		);
	});

	it('remoteWins breaks ties with greater deviceId', () => {
		expect(remoteWins({ modifiedAt: 1, deviceId: 'aaa' }, { modifiedAt: 1, deviceId: 'bbb' })).toBe(
			true
		);
	});

	it('applyRecordLww accepts when no stored row', () => {
		expect(applyRecordLww(null, { modifiedAt: 1, deviceId: 'a' })).toBe('accept');
	});

	it('applyRecordLww rejects stale incoming', () => {
		expect(
			applyRecordLww(
				{ modifiedAt: 200, deviceId: 'b' },
				{ modifiedAt: 100, deviceId: 'z' }
			)
		).toBe('reject');
	});
});
