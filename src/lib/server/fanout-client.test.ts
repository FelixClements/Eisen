const NativeResponse = globalThis.Response;
try {
	new NativeResponse(null, { status: 101 });
} catch {
	const ResponseWithSwitchingProtocols = class extends NativeResponse {
		constructor(body?: BodyInit | null, init?: ResponseInit) {
			if (init?.status === 101) {
				super(body, { ...init, status: 200 });
				Object.defineProperty(this, 'status', { value: 101 });
			} else {
				super(body, init);
			}
		}
	};
	globalThis.Response = ResponseWithSwitchingProtocols as typeof Response;
}

import { describe, expect, it, vi } from 'vitest';
import { FANOUT_ACCOUNT_HEADER, FANOUT_SECRET_HEADER } from './account-fanout';
import { fanoutFromEnv, fanoutWatchBindings, proxyAccountWatch } from './fanout-client';

describe('fanout client', () => {
	it('returns null when the binding or secret is missing', () => {
		expect(fanoutFromEnv(undefined)).toBeNull();
		expect(fanoutFromEnv({ FANOUT_SECRET: 's3cret' } as App.Platform['env'])).toBeNull();
		expect(fanoutWatchBindings(undefined)).toBeNull();
		expect(fanoutWatchBindings({ FANOUT_SECRET: 's3cret' } as App.Platform['env'])).toBeNull();
		expect(fanoutWatchBindings({ SYNC_FANOUT: { fetch: vi.fn() } } as unknown as App.Platform['env'])).toBeNull();
	});

	it('posts notify with the shared secret', async () => {
		const fetch = vi.fn(async () => new Response(null, { status: 204 }));
		const fanout = fanoutFromEnv({
			SYNC_FANOUT: { fetch },
			FANOUT_SECRET: 's3cret'
		} as unknown as App.Platform['env']);
		await fanout?.notify('acct-1', 3);
		expect(fetch).toHaveBeenCalledOnce();
		const init = fetch.mock.calls[0]?.[1] as RequestInit;
		const headers = new Headers(init.headers);
		expect(headers.get(FANOUT_SECRET_HEADER)).toBe('s3cret');
		expect(JSON.parse(String(init.body))).toEqual({ accountId: 'acct-1', version: 3 });
	});

	it('throws when notify is not accepted, so the caller can ignore it', async () => {
		const fanout = fanoutFromEnv({
			SYNC_FANOUT: { fetch: async () => new Response('no', { status: 500 }) },
			FANOUT_SECRET: 's3cret'
		} as unknown as App.Platform['env']);
		await expect(fanout?.notify('acct-1', 3)).rejects.toThrow(/fanout 500/);
	});

	it('proxies the websocket with the session Account id', async () => {
		const fetch = vi.fn(async () => new Response(null, { status: 101 }));
		const request = new Request('https://eisen.example/api/sync/watch', {
			headers: { Upgrade: 'websocket' }
		});
		const response = await proxyAccountWatch(
			{ SYNC_FANOUT: { fetch }, FANOUT_SECRET: 's3cret' },
			request,
			'acct-1'
		);
		expect(response.status).toBe(101);
		const proxied = fetch.mock.calls[0]?.[0] as Request;
		expect(proxied.headers.get(FANOUT_SECRET_HEADER)).toBe('s3cret');
		expect(proxied.headers.get(FANOUT_ACCOUNT_HEADER)).toBe('acct-1');
		expect(proxied.headers.get('Upgrade')).toBe('websocket');
	});

	it('does not forward spoofed fanout headers or browser credentials', async () => {
		const fetch = vi.fn(async () => new Response(null, { status: 101 }));
		const request = new Request('https://eisen.example/api/sync/watch', {
			headers: {
				Upgrade: 'websocket',
				[FANOUT_SECRET_HEADER]: 'spoofed',
				[FANOUT_ACCOUNT_HEADER]: 'spoofed',
				Cookie: 'session=evil',
				Authorization: 'Bearer evil'
			}
		});
		await proxyAccountWatch(
			{ SYNC_FANOUT: { fetch }, FANOUT_SECRET: 's3cret' },
			request,
			'acct-real'
		);
		const proxied = fetch.mock.calls[0]?.[0] as Request;
		expect(proxied.headers.get(FANOUT_SECRET_HEADER)).toBe('s3cret');
		expect(proxied.headers.get(FANOUT_ACCOUNT_HEADER)).toBe('acct-real');
		expect(proxied.headers.get('Cookie')).toBeNull();
		expect(proxied.headers.get('Authorization')).toBeNull();
	});
});
