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
import {
	FANOUT_ACCOUNT_HEADER,
	FANOUT_SECRET_HEADER,
	authorizeFanout,
	changedMessage,
	createAccountFanout,
	routeFanoutRequest
} from './account-fanout';

describe('account fanout', () => {
	it('sends a changed version to every attached socket', () => {
		const hub = createAccountFanout();
		const first: string[] = [];
		const second: string[] = [];
		const a = { send: (data: string) => first.push(data) };
		const b = { send: (data: string) => second.push(data) };
		hub.attach(a);
		hub.attach(b);
		hub.notify(4);
		expect(first).toEqual([changedMessage(4)]);
		expect(second).toEqual([changedMessage(4)]);
		expect(JSON.parse(first[0] ?? '{}')).toEqual({ type: 'changed', version: 4 });
	});

	it('does not send to a detached socket', () => {
		const hub = createAccountFanout();
		const sent: string[] = [];
		const socket = { send: (data: string) => sent.push(data) };
		hub.attach(socket);
		hub.detach(socket);
		hub.notify(2);
		expect(sent).toEqual([]);
	});

	it('rejects a missing or wrong secret', () => {
		expect(authorizeFanout(null, 's3cret')).toBe(false);
		expect(authorizeFanout('nope', 's3cret')).toBe(false);
		expect(authorizeFanout('s3cret', '')).toBe(false);
		expect(authorizeFanout('s3cret', undefined)).toBe(false);
		expect(authorizeFanout('s3cret', 's3cret')).toBe(true);
	});

	it('continues notifying other sockets when one send throws', () => {
		const hub = createAccountFanout();
		const ok: string[] = [];
		const bad = {
			send: () => {
				throw new Error('socket dead');
			}
		};
		const good = { send: (data: string) => ok.push(data) };
		hub.attach(bad);
		hub.attach(good);
		hub.notify(9);
		expect(ok).toEqual([changedMessage(9)]);
	});

	it('rejects a request without the shared secret before touching a stub', async () => {
		const stubFor = vi.fn();
		const response = await routeFanoutRequest(new Request('https://fanout/notify'), {
			secret: 's3cret',
			stubFor
		});
		expect(response.status).toBe(401);
		expect(stubFor).not.toHaveBeenCalled();
	});

	it('returns 401 when the configured secret is missing', async () => {
		const stubFor = vi.fn();
		const response = await routeFanoutRequest(
			new Request('https://fanout/notify', {
				headers: { [FANOUT_SECRET_HEADER]: 's3cret' }
			}),
			{ secret: undefined, stubFor }
		);
		expect(response.status).toBe(401);
		expect(stubFor).not.toHaveBeenCalled();
	});

	it('routes notify to the Account stub', async () => {
		const fetch = vi.fn<(request: Request) => Promise<Response>>(
			async () => new Response(null, { status: 204 })
		);
		const response = await routeFanoutRequest(
			new Request('https://fanout/notify', {
				method: 'POST',
				headers: { [FANOUT_SECRET_HEADER]: 's3cret', 'Content-Type': 'application/json' },
				body: JSON.stringify({ accountId: 'acct-1', version: 7 })
			}),
			{ secret: 's3cret', stubFor: () => ({ fetch }) }
		);
		expect(response.status).toBe(204);
		expect(fetch).toHaveBeenCalledOnce();
		const forwarded = fetch.mock.calls[0]![0];
		expect(new URL(forwarded.url).pathname).toBe('/notify');
		expect(await forwarded.json()).toEqual({ version: 7 });
	});

	it('routes a websocket to the session Account', async () => {
		const fetch = vi.fn(async () => new Response(null, { status: 101 }));
		const stubFor = vi.fn(() => ({ fetch }));
		const request = new Request('https://eisen.example/api/sync/watch', {
			headers: {
				[FANOUT_SECRET_HEADER]: 's3cret',
				[FANOUT_ACCOUNT_HEADER]: 'acct-9',
				Upgrade: 'websocket'
			}
		});
		const response = await routeFanoutRequest(request, { secret: 's3cret', stubFor });
		expect(response.status).toBe(101);
		expect(stubFor).toHaveBeenCalledWith('acct-9');
		expect(fetch).toHaveBeenCalledWith(request);
	});

	it('rejects a websocket without an Account id', async () => {
		const stubFor = vi.fn();
		const response = await routeFanoutRequest(
			new Request('https://eisen.example/api/sync/watch', {
				headers: { [FANOUT_SECRET_HEADER]: 's3cret', Upgrade: 'websocket' }
			}),
			{ secret: 's3cret', stubFor }
		);
		expect(response.status).toBe(400);
		expect(stubFor).not.toHaveBeenCalled();
	});
});
