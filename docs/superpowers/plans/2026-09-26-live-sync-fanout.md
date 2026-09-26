# Live Sync Fanout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** While a Workspace is open and the tab is visible, a Task change accepted on another device shows up within a second or two, without pressing Sync now.

**Architecture:** Task blobs still move only through `POST /api/sync`. After an exchange advances the Account sync version, Pages tells a Durable Object for that Account. The object sends `{ "type": "changed", "version": <number> }` to every visible Workspace. `sync-watch` turns that ping into the existing `syncEngine.run()`, and runs one Sync when the tab becomes visible again.

**Tech Stack:** SvelteKit on Cloudflare Pages, Vitest in Node, a second Worker with one Durable Object per Account, hibernated WebSockets.

## Global Constraints

- Task blobs move only through `POST /api/sync`. The HTTP body stays `{ changes, lastVersion }`.
- The live message is exactly `{ "type": "changed", "version": <number> }`. No Task plaintext or ciphertext.
- The socket stays up only while the Workspace is open and `document.visibilityState` is `visible`.
- Hiding the tab closes the socket. Showing the tab runs one Sync, then opens the socket. Nothing syncs in the background.
- Reconnect delays while visible are 1s, 2s, 4s, 8s, 16s, then 30s.
- A 401 stops the reconnect timer and the Workspace sets `sync.lastError` to `{ "code": "session-expired" }`.
- A missing fanout binding makes `GET /api/sync/watch` return 404. `sync-watch` does not reconnect on a timer. The next visibility change tries once.
- Notify failure, or a missing binding, does not change the Sync response.
- An empty pull, or a last-write-wins rejection, does not ping anyone.
- Headers are `X-Fanout-Secret` and `X-Account-Id`. The shared secret env var is `FANOUT_SECRET`.
- Worker name is `eisen-sync-fanout`. Internal notify is `POST /notify` with `{ accountId, version }`.
- Browser route is `GET /api/sync/watch`. The Account id comes from the session.
- Merge stays whole-record last-write-wins on `(updatedAt, deviceId)`. Do not edit ADR-013.
- Automated tests use Vitest and fakes. Do not open a real Durable Object.

---

### Task 1: Account fanout hub

**Files:**
- Create: `src/lib/server/account-fanout.ts`
- Test: `src/lib/server/account-fanout.test.ts`

**Interfaces:**
- Consumes: nothing
- Produces:
  - `FANOUT_SECRET_HEADER = "X-Fanout-Secret"`
  - `FANOUT_ACCOUNT_HEADER = "X-Account-Id"`
  - `changedMessage(version: number): string`
  - `authorizeFanout(header: string | null, secret: string): boolean`
  - `FanoutSocket = { send(data: string): void }`
  - `createAccountFanout(): { attach(socket: FanoutSocket): void; detach(socket: FanoutSocket): void; notify(version: number): void }`
  - `FanoutStub = { fetch(request: Request): Promise<Response> }`
  - `routeFanoutRequest(request: Request, opts: { secret: string; stubFor(accountId: string): FanoutStub }): Promise<Response>`

- [ ] **Step 1: Write the failing test**

```ts
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
		expect(authorizeFanout('s3cret', 's3cret')).toBe(true);
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

	it('routes notify to the Account stub', async () => {
		const fetch = vi.fn(async () => new Response(null, { status: 204 }));
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
		const forwarded = fetch.mock.calls[0]?.[0] as Request;
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
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npx vitest run src/lib/server/account-fanout.test.ts`

Expected: FAIL because `src/lib/server/account-fanout.ts` does not exist.

- [ ] **Step 3: Write the minimal implementation**

```ts
export const FANOUT_SECRET_HEADER = 'X-Fanout-Secret';
export const FANOUT_ACCOUNT_HEADER = 'X-Account-Id';

export function changedMessage(version: number): string {
	return JSON.stringify({ type: 'changed', version });
}

export function authorizeFanout(header: string | null, secret: string): boolean {
	return secret.length > 0 && header === secret;
}

export type FanoutSocket = {
	send(data: string): void;
};

export function createAccountFanout() {
	const sockets = new Set<FanoutSocket>();
	return {
		attach(socket: FanoutSocket) {
			sockets.add(socket);
		},
		detach(socket: FanoutSocket) {
			sockets.delete(socket);
		},
		notify(version: number) {
			const message = changedMessage(version);
			for (const socket of sockets) socket.send(message);
		}
	};
}

export type FanoutStub = {
	fetch(request: Request): Promise<Response>;
};

export async function routeFanoutRequest(
	request: Request,
	opts: { secret: string; stubFor(accountId: string): FanoutStub }
): Promise<Response> {
	if (!authorizeFanout(request.headers.get(FANOUT_SECRET_HEADER), opts.secret)) {
		return new Response('unauthorized', { status: 401 });
	}
	const url = new URL(request.url);
	if (url.pathname === '/notify' && request.method === 'POST') {
		const body = (await request.json()) as { accountId?: string; version?: number };
		if (!body.accountId || typeof body.version !== 'number') {
			return new Response('bad notify', { status: 400 });
		}
		return opts.stubFor(body.accountId).fetch(
			new Request('https://fanout/notify', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ version: body.version })
			})
		);
	}
	if (request.headers.get('Upgrade') !== 'websocket') {
		return new Response('expected websocket', { status: 426 });
	}
	const accountId = request.headers.get(FANOUT_ACCOUNT_HEADER);
	if (!accountId) return new Response('missing account', { status: 400 });
	return opts.stubFor(accountId).fetch(request);
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `npx vitest run src/lib/server/account-fanout.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/server/account-fanout.ts src/lib/server/account-fanout.test.ts
git commit -m "$(cat <<'EOF'
Add an in-memory Account fanout for version pings.

EOF
)"
```

---

### Task 2: sync-watch

**Files:**
- Create: `src/lib/workspace/sync-watch.ts`
- Test: `src/lib/workspace/sync-watch.test.ts`

**Interfaces:**
- Consumes: nothing from Task 1
- Produces:
  - `WatchSocket = { onMessage(handler: (data: string) => void): void; onClose(handler: () => void): void; close(): void }`
  - `WatchConnectResult = { ok: true; socket: WatchSocket } | { ok: false; status: 401 | 404 | 0 }`
  - `SyncWatchHooks = { runSync(): Promise<void>; getLastVersion(): number; sessionExpired(): boolean; onSessionExpired(): void }`
  - `SyncWatch = { start(): void; stop(): void }`
  - `createSyncWatch(deps: SyncWatchDeps): SyncWatch` where `SyncWatchDeps` is `SyncWatchHooks` plus `visibility`, `online`, `connect`, `setTimer`, and `clearTimer` as written below

Behavior the tests lock in:

- `start()` while visible connects and does not Sync. The Workspace already ran the opening Sync.
- `start()` while hidden does not connect.
- Hiding closes the socket and clears the reconnect timer.
- Showing the tab runs one Sync, then connects.
- A `changed` version newer than `getLastVersion()` runs Sync. An equal, older, or unparseable message does not.
- If that Sync returns with the version still unchanged, run Sync once more. If the version is still unchanged, stop. That second call is the pull that a joined in-flight Sync missed. A failed Sync must not spin.
- If the first Sync already reaches the ping version, there is no second call.
- A socket close while visible schedules 1000ms, then connects and runs one Sync.
- 401 calls `onSessionExpired` and does not schedule a timer. A later visibility change may try again.
- 404 does not schedule a timer. The next visibility change tries again.
- `sessionExpired()` true after Sync clears the timer and closes the socket.
- `stop()` closes the socket and clears the timer.

- [ ] **Step 1: Write the failing test**

```ts
import { describe, expect, it } from 'vitest';
import { createSyncWatch, type WatchConnectResult, type WatchSocket } from './sync-watch';

function harness(initial: 'visible' | 'hidden' = 'visible') {
	let visibility = initial;
	let online = true;
	const visListeners = new Set<() => void>();
	const onlineListeners = new Set<() => void>();
	const timers = new Map<number, { fn: () => void; ms: number }>();
	let nextTimer = 1;
	let version = 1;
	let calls = 0;
	let session = false;
	let expired = 0;
	const sockets: FakeSocket[] = [];
	const connects: number[] = [];

	class FakeSocket implements WatchSocket {
		onMessageHandler: (data: string) => void = () => {};
		onCloseHandler: () => void = () => {};
		closed = false;
		onMessage(handler: (data: string) => void) {
			this.onMessageHandler = handler;
		}
		onClose(handler: () => void) {
			this.onCloseHandler = handler;
		}
		close() {
			this.closed = true;
			this.onCloseHandler();
		}
		message(data: string) {
			this.onMessageHandler(data);
		}
	}

	const watch = createSyncWatch({
		visibility: {
			get state() {
				return visibility;
			},
			subscribe(listener) {
				visListeners.add(listener);
				return () => visListeners.delete(listener);
			}
		},
		online: {
			get online() {
				return online;
			},
			subscribe(listener) {
				onlineListeners.add(listener);
				return () => onlineListeners.delete(listener);
			}
		},
		connect: async (): Promise<WatchConnectResult> => {
			connects.push(connects.length + 1);
			const socket = new FakeSocket();
			sockets.push(socket);
			return { ok: true, socket };
		},
		runSync: async () => {
			calls += 1;
		},
		getLastVersion: () => version,
		sessionExpired: () => session,
		onSessionExpired: () => {
			expired += 1;
		},
		setTimer(fn, ms) {
			const id = nextTimer++;
			timers.set(id, { fn, ms });
			return id;
		},
		clearTimer(id) {
			timers.delete(id);
		}
	});

	return {
		watch,
		sockets,
		connects,
		get calls() {
			return calls;
		},
		get expired() {
			return expired;
		},
		get timerCount() {
			return timers.size;
		},
		timerMs() {
			return [...timers.values()].map((t) => t.ms);
		},
		fireTimer() {
			const entry = [...timers.entries()][0];
			if (!entry) throw new Error('no timer');
			timers.delete(entry[0]);
			entry[1].fn();
		},
		setVersion(next: number) {
			version = next;
		},
		failSession() {
			session = true;
		},
		setVisibility(next: 'visible' | 'hidden') {
			visibility = next;
			for (const listener of visListeners) listener();
		},
		setOnline(next: boolean) {
			online = next;
			for (const listener of onlineListeners) listener();
		},
		async settle() {
			await Promise.resolve();
			await Promise.resolve();
		}
	};
}

describe('sync-watch', () => {
	it('connects when started visible and does not sync', async () => {
		const h = harness();
		h.watch.start();
		await h.settle();
		expect(h.connects).toEqual([1]);
		expect(h.calls).toBe(0);
	});

	it('does not connect when started hidden', async () => {
		const h = harness('hidden');
		h.watch.start();
		await h.settle();
		expect(h.connects).toEqual([]);
	});

	it('closes the socket when the tab hides', async () => {
		const h = harness();
		h.watch.start();
		await h.settle();
		h.setVisibility('hidden');
		expect(h.sockets[0]?.closed).toBe(true);
		expect(h.timerCount).toBe(0);
	});

	it('syncs once and then connects when the tab is shown', async () => {
		const order: string[] = [];
		let state: 'visible' | 'hidden' = 'hidden';
		const listeners = new Set<() => void>();
		const watch = createSyncWatch({
			visibility: {
				get state() {
					return state;
				},
				subscribe(listener) {
					listeners.add(listener);
					return () => listeners.delete(listener);
				}
			},
			online: { get online() { return true; }, subscribe: () => () => {} },
			connect: async () => {
				order.push('connect');
				return { ok: true, socket: { onMessage() {}, onClose() {}, close() {} } };
			},
			runSync: async () => {
				order.push('sync');
			},
			getLastVersion: () => 1,
			sessionExpired: () => false,
			onSessionExpired: () => {},
			setTimer: () => 1,
			clearTimer: () => {}
		});
		watch.start();
		await Promise.resolve();
		state = 'visible';
		for (const listener of listeners) listener();
		await Promise.resolve();
		await Promise.resolve();
		expect(order).toEqual(['sync', 'connect']);
	});

	it('runs sync for a newer changed version only', async () => {
		const h = harness();
		h.watch.start();
		await h.settle();
		h.sockets[0]?.message(JSON.stringify({ type: 'changed', version: 1 }));
		h.sockets[0]?.message('nope');
		await h.settle();
		expect(h.calls).toBe(0);
		h.sockets[0]?.message(JSON.stringify({ type: 'changed', version: 2 }));
		await h.settle();
		expect(h.calls).toBe(1);
	});

	it('does not run a second sync when the first reaches the ping', async () => {
		let version = 1;
		let calls = 0;
		let current: ((data: string) => void) | null = null;
		const watch = createSyncWatch({
			visibility: { get state() { return 'visible' as const; }, subscribe: () => () => {} },
			online: { get online() { return true; }, subscribe: () => () => {} },
			connect: async () => ({
				ok: true,
				socket: {
					onMessage(handler) {
						current = handler;
					},
					onClose() {},
					close() {}
				}
			}),
			runSync: async () => {
				calls += 1;
				version = 4;
			},
			getLastVersion: () => version,
			sessionExpired: () => false,
			onSessionExpired: () => {},
			setTimer: () => 1,
			clearTimer: () => {}
		});
		watch.start();
		await Promise.resolve();
		await Promise.resolve();
		current?.(JSON.stringify({ type: 'changed', version: 4 }));
		await Promise.resolve();
		await Promise.resolve();
		expect(calls).toBe(1);
	});

	it('runs one more sync when the in-flight sync does not advance', async () => {
		let version = 1;
		let calls = 0;
		let release: () => void = () => {};
		const gate = new Promise<void>((resolve) => {
			release = resolve;
		});
		let current: ((data: string) => void) | null = null;
		const watch = createSyncWatch({
			visibility: { get state() { return 'visible' as const; }, subscribe: () => () => {} },
			online: { get online() { return true; }, subscribe: () => () => {} },
			connect: async () => ({
				ok: true,
				socket: {
					onMessage(handler) {
						current = handler;
					},
					onClose() {},
					close() {}
				}
			}),
			runSync: async () => {
				calls += 1;
				if (calls === 1) {
					await gate;
					return;
				}
				version = 5;
			},
			getLastVersion: () => version,
			sessionExpired: () => false,
			onSessionExpired: () => {},
			setTimer: () => 1,
			clearTimer: () => {}
		});
		watch.start();
		await Promise.resolve();
		await Promise.resolve();
		current?.(JSON.stringify({ type: 'changed', version: 5 }));
		await Promise.resolve();
		expect(calls).toBe(1);
		release();
		await gate;
		await Promise.resolve();
		await Promise.resolve();
		expect(calls).toBe(2);
		expect(version).toBe(5);
	});

	it('reconnects after a drop and syncs once', async () => {
		const h = harness();
		h.watch.start();
		await h.settle();
		h.sockets[0]?.close();
		expect(h.timerMs()).toEqual([1000]);
		h.fireTimer();
		await h.settle();
		expect(h.connects).toEqual([1, 2]);
		expect(h.calls).toBe(1);
	});

	it('stops the timer on 401 and reports session-expired', async () => {
		let expired = 0;
		const timers: number[] = [];
		const watch = createSyncWatch({
			visibility: { get state() { return 'visible' as const; }, subscribe: () => () => {} },
			online: { get online() { return true; }, subscribe: () => () => {} },
			connect: async () => ({ ok: false, status: 401 }),
			runSync: async () => {},
			getLastVersion: () => 1,
			sessionExpired: () => false,
			onSessionExpired: () => {
				expired += 1;
			},
			setTimer: (_fn, ms) => {
				timers.push(ms);
				return 1;
			},
			clearTimer: () => {}
		});
		watch.start();
		await Promise.resolve();
		await Promise.resolve();
		expect(expired).toBe(1);
		expect(timers).toEqual([]);
	});

	it('does not arm a timer on 404, and tries again when shown', async () => {
		let visibility: 'visible' | 'hidden' = 'visible';
		const listeners = new Set<() => void>();
		let connects = 0;
		const timers: number[] = [];
		const watch = createSyncWatch({
			visibility: {
				get state() {
					return visibility;
				},
				subscribe(listener) {
					listeners.add(listener);
					return () => listeners.delete(listener);
				}
			},
			online: { get online() { return true; }, subscribe: () => () => {} },
			connect: async () => {
				connects += 1;
				return { ok: false, status: 404 };
			},
			runSync: async () => {},
			getLastVersion: () => 1,
			sessionExpired: () => false,
			onSessionExpired: () => {},
			setTimer: () => {
				timers.push(1);
				return 1;
			},
			clearTimer: () => {}
		});
		watch.start();
		await Promise.resolve();
		await Promise.resolve();
		expect(connects).toBe(1);
		expect(timers).toEqual([]);
		visibility = 'hidden';
		for (const listener of listeners) listener();
		visibility = 'visible';
		for (const listener of listeners) listener();
		await Promise.resolve();
		await Promise.resolve();
		expect(connects).toBe(2);
	});

	it('closes and clears timers on stop', async () => {
		const h = harness();
		h.watch.start();
		await h.settle();
		h.sockets[0]?.close();
		expect(h.timerCount).toBe(1);
		h.watch.stop();
		expect(h.timerCount).toBe(0);
		expect(h.sockets[0]?.closed).toBe(true);
	});

	it('waits 1000ms and then 2000ms when reconnect keeps failing', async () => {
		let attempt = 0;
		const timers: Array<{ fn: () => void; ms: number }> = [];
		const watch = createSyncWatch({
			visibility: { get state() { return 'visible' as const; }, subscribe: () => () => {} },
			online: { get online() { return true; }, subscribe: () => () => {} },
			connect: async () => {
				attempt += 1;
				if (attempt === 1) {
					return {
						ok: true,
						socket: {
							onMessage() {},
							onClose(handler) {
								handler();
							},
							close() {}
						}
					};
				}
				return { ok: false, status: 0 };
			},
			runSync: async () => {},
			getLastVersion: () => 1,
			sessionExpired: () => false,
			onSessionExpired: () => {},
			setTimer(fn, ms) {
				timers.push({ fn, ms });
				return timers.length;
			},
			clearTimer(id) {
				timers.splice(id - 1, 1);
			}
		});
		watch.start();
		await Promise.resolve();
		await Promise.resolve();
		expect(timers.map((t) => t.ms)).toEqual([1000]);
		timers[0]?.fn();
		await Promise.resolve();
		await Promise.resolve();
		expect(timers.map((t) => t.ms)).toEqual([1000, 2000]);
	});

	it('syncs and connects when the browser comes back online', async () => {
		const order: string[] = [];
		let online = false;
		const listeners = new Set<() => void>();
		const watch = createSyncWatch({
			visibility: { get state() { return 'visible' as const; }, subscribe: () => () => {} },
			online: {
				get online() {
					return online;
				},
				subscribe(listener) {
					listeners.add(listener);
					return () => listeners.delete(listener);
				}
			},
			connect: async () => {
				order.push('connect');
				return { ok: true, socket: { onMessage() {}, onClose() {}, close() {} } };
			},
			runSync: async () => {
				order.push('sync');
			},
			getLastVersion: () => 1,
			sessionExpired: () => false,
			onSessionExpired: () => {},
			setTimer: () => 1,
			clearTimer: () => {}
		});
		watch.start();
		await Promise.resolve();
		await Promise.resolve();
		order.length = 0;
		online = true;
		for (const listener of listeners) listener();
		await Promise.resolve();
		await Promise.resolve();
		expect(order).toEqual(['sync', 'connect']);
	});

	it('stops reconnecting when sync reports session-expired', async () => {
		let session = false;
		let expired = 0;
		const timers: number[] = [];
		let current: ((data: string) => void) | null = null;
		const watch = createSyncWatch({
			visibility: { get state() { return 'visible' as const; }, subscribe: () => () => {} },
			online: { get online() { return true; }, subscribe: () => () => {} },
			connect: async () => ({
				ok: true,
				socket: {
					onMessage(handler) {
						current = handler;
					},
					onClose() {},
					close() {}
				}
			}),
			runSync: async () => {
				session = true;
			},
			getLastVersion: () => 1,
			sessionExpired: () => session,
			onSessionExpired: () => {
				expired += 1;
			},
			setTimer(_fn, ms) {
				timers.push(ms);
				return 1;
			},
			clearTimer: () => {}
		});
		watch.start();
		await Promise.resolve();
		await Promise.resolve();
		current?.(JSON.stringify({ type: 'changed', version: 9 }));
		await Promise.resolve();
		await Promise.resolve();
		expect(expired).toBe(1);
		expect(timers).toEqual([]);
	});
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npx vitest run src/lib/workspace/sync-watch.test.ts`

Expected: FAIL because `src/lib/workspace/sync-watch.ts` does not exist.

- [ ] **Step 3: Write the minimal implementation**

Implementation:

```ts
const BACKOFF_MS = [1000, 2000, 4000, 8000, 16000, 30000] as const;

export type WatchSocket = {
	onMessage(handler: (data: string) => void): void;
	onClose(handler: () => void): void;
	close(): void;
};

export type WatchConnectResult =
	| { ok: true; socket: WatchSocket }
	| { ok: false; status: 401 | 404 | 0 };

export type SyncWatchHooks = {
	runSync(): Promise<void>;
	getLastVersion(): number;
	sessionExpired(): boolean;
	onSessionExpired(): void;
};

export type SyncWatchDeps = SyncWatchHooks & {
	visibility: {
		readonly state: 'visible' | 'hidden';
		subscribe(listener: () => void): () => void;
	};
	online: {
		readonly online: boolean;
		subscribe(listener: () => void): () => void;
	};
	connect(): Promise<WatchConnectResult>;
	setTimer(callback: () => void, ms: number): number;
	clearTimer(id: number): void;
};

export type SyncWatch = {
	start(): void;
	stop(): void;
};

function readChangedVersion(data: string): number | null {
	try {
		const message = JSON.parse(data) as { type?: string; version?: unknown };
		if (message.type !== 'changed' || typeof message.version !== 'number') return null;
		return message.version;
	} catch {
		return null;
	}
}

export function createSyncWatch(deps: SyncWatchDeps): SyncWatch {
	let started = false;
	let stopped = false;
	let socket: WatchSocket | null = null;
	let timer: number | null = null;
	let attempt = 0;
	let generation = 0;
	let newestPing = 0;
	let pumping = false;
	let pumpAgain = false;
	let unsubVisibility = () => {};
	let unsubOnline = () => {};

	function clearReconnect() {
		if (timer === null) return;
		deps.clearTimer(timer);
		timer = null;
	}

	function closeSocket() {
		const current = socket;
		socket = null;
		current?.close();
	}

	function haltForSession(): boolean {
		if (!deps.sessionExpired()) return false;
		deps.onSessionExpired();
		clearReconnect();
		closeSocket();
		return true;
	}

	function noteVersion(version: number) {
		if (stopped || version <= deps.getLastVersion()) return;
		if (version > newestPing) newestPing = version;
		void pump();
	}

	async function pump() {
		if (stopped || pumping) {
			if (pumping) pumpAgain = true;
			return;
		}
		pumping = true;
		try {
			do {
				pumpAgain = false;
				while (!stopped && newestPing > deps.getLastVersion()) {
					const before = deps.getLastVersion();
					await deps.runSync();
					if (stopped || haltForSession()) return;
					if (deps.getLastVersion() === before) {
						await deps.runSync();
						if (stopped || haltForSession()) return;
						if (deps.getLastVersion() === before) return;
					}
				}
			} while (!stopped && (pumpAgain || newestPing > deps.getLastVersion()));
		} finally {
			pumping = false;
		}
	}

	function scheduleReconnect() {
		if (stopped || !started || deps.visibility.state !== 'visible') return;
		clearReconnect();
		const delay = BACKOFF_MS[Math.min(attempt, BACKOFF_MS.length - 1)] ?? 30000;
		attempt += 1;
		timer = deps.setTimer(() => {
			timer = null;
			void open({ syncFirst: false, syncOnConnect: true });
		}, delay);
	}

	async function open(opts: { syncFirst: boolean; syncOnConnect: boolean }) {
		if (stopped || !started || deps.visibility.state !== 'visible') return;
		const my = ++generation;
		if (opts.syncFirst) {
			await deps.runSync();
			if (stopped || my !== generation) return;
			if (haltForSession()) return;
		}
		if (deps.visibility.state !== 'visible') return;
		const result = await deps.connect();
		if (stopped || my !== generation) {
			if (result.ok) result.socket.close();
			return;
		}
		if (!result.ok) {
			if (result.status === 401) {
				deps.onSessionExpired();
				clearReconnect();
				return;
			}
			if (result.status === 404) {
				clearReconnect();
				return;
			}
			scheduleReconnect();
			return;
		}
		attempt = 0;
		closeSocket();
		socket = result.socket;
		socket.onMessage((data) => {
			const version = readChangedVersion(data);
			if (version !== null) noteVersion(version);
		});
		socket.onClose(() => {
			if (socket !== result.socket) return;
			socket = null;
			if (stopped || deps.visibility.state !== 'visible') return;
			scheduleReconnect();
		});
		if (opts.syncOnConnect) {
			await deps.runSync();
			if (haltForSession()) return;
		}
	}

	return {
		start() {
			if (started) return;
			started = true;
			stopped = false;
			unsubVisibility = deps.visibility.subscribe(() => {
				if (stopped) return;
				if (deps.visibility.state === 'hidden') {
					generation += 1;
					clearReconnect();
					closeSocket();
					return;
				}
				clearReconnect();
				attempt = 0;
				void open({ syncFirst: true, syncOnConnect: false });
			});
			unsubOnline = deps.online.subscribe(() => {
				if (stopped || !deps.online.online || deps.visibility.state !== 'visible') return;
				clearReconnect();
				attempt = 0;
				void open({ syncFirst: true, syncOnConnect: false });
			});
			if (deps.visibility.state === 'visible') {
				void open({ syncFirst: false, syncOnConnect: false });
			}
		},
		stop() {
			stopped = true;
			started = false;
			generation += 1;
			unsubVisibility();
			unsubOnline();
			unsubVisibility = () => {};
			unsubOnline = () => {};
			clearReconnect();
			closeSocket();
		}
	};
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `npx vitest run src/lib/workspace/sync-watch.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/workspace/sync-watch.ts src/lib/workspace/sync-watch.test.ts
git commit -m "$(cat <<'EOF'
Listen for Sync version pings while the Workspace tab is visible.

EOF
)"
```

---

### Task 3: Notify only when the sync version advances

**Files:**
- Modify: `src/lib/server/mirror/record-sync.ts`
- Modify: `src/lib/server/mirror/types.ts` (`EncryptedMirror` type, the `exchangeSync` line)
- Create: `src/lib/server/sync-notify.ts`
- Test: `src/lib/server/sync-notify.test.ts`

**Interfaces:**
- Consumes: `EncryptedMirror.exchangeSync` from `src/lib/server/encrypted-mirror.ts`
- Produces:
  - `EncryptedMirror.maxSyncVersion(accountId: string): Promise<number>`
  - `FanoutNotify = { notify(accountId: string, version: number): Promise<void> }`
  - `exchangeAndNotify(mirror: VersionedSync, fanout: FanoutNotify | null, accountId: string, batch: SyncPushBatch): Promise<SyncPullBatch>`
  - `VersionedSync = { maxSyncVersion(accountId: string): Promise<number>; exchangeSync(accountId: string, batch: SyncPushBatch): Promise<SyncPullBatch> }`

- [ ] **Step 1: Write the failing test**

```ts
import { describe, expect, it, vi } from 'vitest';
import { createEncryptedMirror } from './encrypted-mirror';
import { memoryMirrorDatabase, memoryRecoveryObjects, recordingPushDispatch } from './adapters/memory';
import { exchangeAndNotify } from './sync-notify';
import type { SyncPushBatch } from '$lib/sync/types';

const ACCOUNT = 'acct-1';

function mirror() {
	return createEncryptedMirror({
		database: memoryMirrorDatabase(),
		recoveryObjects: memoryRecoveryObjects(),
		pushDispatch: recordingPushDispatch()
	});
}

function push(modifiedAt: number, blob: string): SyncPushBatch {
	return {
		lastVersion: 0,
		changes: [
			{
				recordId: 'r1',
				encryptedBlob: blob,
				modifiedAt,
				deviceId: 'device-b',
				deleted: 0
			}
		]
	};
}

describe('exchangeAndNotify', () => {
	it('notifies with the version a write advances to', async () => {
		const store = mirror();
		const notify = vi.fn(async () => {});
		const result = await exchangeAndNotify(store, { notify }, ACCOUNT, push(200, 'blob-new'));
		expect(result.lastVersion).toBeGreaterThan(0);
		expect(notify).toHaveBeenCalledWith(ACCOUNT, result.lastVersion);
		expect(result.changes[0]?.encryptedBlob).toBe('blob-new');
	});

	it('does not notify on an empty pull', async () => {
		const store = mirror();
		const notify = vi.fn(async () => {});
		await exchangeAndNotify(store, { notify }, ACCOUNT, { lastVersion: 0, changes: [] });
		expect(notify).not.toHaveBeenCalled();
	});

	it('does not notify when last-write-wins rejects the push', async () => {
		const store = mirror();
		await store.exchangeSync(ACCOUNT, push(200, 'blob-new'));
		const notify = vi.fn(async () => {});
		await exchangeAndNotify(store, { notify }, ACCOUNT, push(100, 'blob-old'));
		expect(notify).not.toHaveBeenCalled();
	});

	it('returns the exchange when notify throws', async () => {
		const store = mirror();
		const result = await exchangeAndNotify(
			store,
			{
				notify: async () => {
					throw new Error('fanout down');
				}
			},
			ACCOUNT,
			push(200, 'blob-new')
		);
		expect(result.changes[0]?.encryptedBlob).toBe('blob-new');
		expect(result.lastVersion).toBeGreaterThan(0);
	});

	it('returns the exchange when fanout is missing', async () => {
		const store = mirror();
		const result = await exchangeAndNotify(store, null, ACCOUNT, push(200, 'blob-new'));
		expect(result.lastVersion).toBeGreaterThan(0);
	});
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npx vitest run src/lib/server/sync-notify.test.ts`

Expected: FAIL because `exchangeAndNotify` is not defined, and `maxSyncVersion` is not on the mirror yet. The reject test can call `exchangeSync` directly for setup; only the notify path needs the new function.

- [ ] **Step 3: Write the minimal implementation**

In `src/lib/server/mirror/record-sync.ts`, add this method on the returned object next to `exchangeSync`:

```ts
async maxSyncVersion(accountId: string): Promise<number> {
	return database.maxSyncVersion(accountId);
}
```

In `src/lib/server/mirror/types.ts`, add this line to `EncryptedMirror` immediately after `exchangeSync`:

```ts
maxSyncVersion(accountId: string): Promise<number>;
```

Create `src/lib/server/sync-notify.ts`:

```ts
import type { SyncPullBatch, SyncPushBatch } from '$lib/sync/types';

export type VersionedSync = {
	maxSyncVersion(accountId: string): Promise<number>;
	exchangeSync(accountId: string, batch: SyncPushBatch): Promise<SyncPullBatch>;
};

export type FanoutNotify = {
	notify(accountId: string, version: number): Promise<void>;
};

export async function exchangeAndNotify(
	mirror: VersionedSync,
	fanout: FanoutNotify | null,
	accountId: string,
	batch: SyncPushBatch
): Promise<SyncPullBatch> {
	const before = await mirror.maxSyncVersion(accountId);
	const result = await mirror.exchangeSync(accountId, batch);
	const after = await mirror.maxSyncVersion(accountId);
	if (fanout && after > before) {
		try {
			await fanout.notify(accountId, after);
		} catch {
			// The blobs are already stored. A missed ping is repaired when the tab becomes visible.
		}
	}
	return result;
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `npx vitest run src/lib/server/sync-notify.test.ts src/lib/server/encrypted-mirror.test.ts`

Expected: PASS. Existing mirror tests still pass because `maxSyncVersion` is additive.

- [ ] **Step 5: Commit**

```bash
git add src/lib/server/mirror/record-sync.ts src/lib/server/mirror/types.ts src/lib/server/sync-notify.ts src/lib/server/sync-notify.test.ts
git commit -m "$(cat <<'EOF'
Ping the fanout only when an exchange advances the sync version.

EOF
)"
```

---

### Task 4: Pages Sync route and watch proxy

**Files:**
- Modify: `src/routes/api/sync/+server.ts`
- Create: `src/routes/api/sync/watch/+server.ts`
- Create: `src/lib/server/fanout-client.ts`
- Test: `src/lib/server/fanout-client.test.ts`
- Modify: `src/app.d.ts` (`Platform.env`)
- Modify: `wrangler.toml` (service binding and secret comment)

**Interfaces:**
- Consumes: `exchangeAndNotify` and `FanoutNotify` from Task 3. `FANOUT_SECRET_HEADER` and `FANOUT_ACCOUNT_HEADER` from Task 1.
- Produces:
  - `fanoutFromEnv(env: App.Platform['env'] | undefined): FanoutNotify | null`
  - `proxyAccountWatch(env: { SYNC_FANOUT: { fetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response> }; FANOUT_SECRET: string }, request: Request, accountId: string): Promise<Response>`
  - `GET /api/sync/watch` returns 404 when `SYNC_FANOUT` or `FANOUT_SECRET` is missing, 204 when the probe has no `Upgrade: websocket`, and otherwise the proxied WebSocket response.
  - `POST /api/sync` calls `exchangeAndNotify` and returns `json(result)`.

- [ ] **Step 1: Write the failing test**

```ts
import { describe, expect, it, vi } from 'vitest';
import { FANOUT_ACCOUNT_HEADER, FANOUT_SECRET_HEADER } from './account-fanout';
import { fanoutFromEnv, proxyAccountWatch } from './fanout-client';

describe('fanout client', () => {
	it('returns null when the binding or secret is missing', () => {
		expect(fanoutFromEnv(undefined)).toBeNull();
		expect(fanoutFromEnv({ FANOUT_SECRET: 's3cret' } as App.Platform['env'])).toBeNull();
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
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npx vitest run src/lib/server/fanout-client.test.ts`

Expected: FAIL because `src/lib/server/fanout-client.ts` does not exist.

- [ ] **Step 3: Write the minimal implementation**

Create `src/lib/server/fanout-client.ts`:

```ts
import { FANOUT_ACCOUNT_HEADER, FANOUT_SECRET_HEADER } from './account-fanout';
import type { FanoutNotify } from './sync-notify';

type FanoutFetcher = {
	fetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response>;
};

export function fanoutFromEnv(env: App.Platform['env'] | undefined): FanoutNotify | null {
	if (!env?.SYNC_FANOUT || !env.FANOUT_SECRET) return null;
	const binding = env.SYNC_FANOUT;
	const secret = env.FANOUT_SECRET;
	return {
		async notify(accountId, version) {
			const response = await binding.fetch('https://fanout/notify', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					[FANOUT_SECRET_HEADER]: secret
				},
				body: JSON.stringify({ accountId, version })
			});
			if (!response.ok) throw new Error(`fanout ${response.status}`);
		}
	};
}

export async function proxyAccountWatch(
	env: { SYNC_FANOUT: FanoutFetcher; FANOUT_SECRET: string },
	request: Request,
	accountId: string
): Promise<Response> {
	const headers = new Headers(request.headers);
	headers.set(FANOUT_SECRET_HEADER, env.FANOUT_SECRET);
	headers.set(FANOUT_ACCOUNT_HEADER, accountId);
	return env.SYNC_FANOUT.fetch(new Request(request, { headers }));
}
```

In `src/app.d.ts`, add these two optional fields inside `Platform.env`:

```ts
SYNC_FANOUT?: {
	fetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response>;
};
FANOUT_SECRET?: string;
```

Replace `src/routes/api/sync/+server.ts` with:

```ts
import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { requireUser } from '$lib/server/require-user';
import { mirrorFromEvent } from '$lib/server/mirror-from-event';
import { fanoutFromEnv } from '$lib/server/fanout-client';
import { exchangeAndNotify } from '$lib/server/sync-notify';
import type { SyncRecord } from '$lib/sync/types';

export const POST: RequestHandler = async (event) => {
	const user = requireUser(event);
	const mirror = mirrorFromEvent(event);
	const body = (await event.request.json()) as {
		lastVersion: number;
		changes: SyncRecord[];
	};
	const result = await exchangeAndNotify(
		mirror,
		fanoutFromEnv(event.platform?.env),
		user.id,
		{
			lastVersion: body.lastVersion ?? 0,
			changes: body.changes ?? []
		}
	);
	return json(result);
};
```

Create `src/routes/api/sync/watch/+server.ts`:

```ts
import { error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { requireUser } from '$lib/server/require-user';
import { proxyAccountWatch } from '$lib/server/fanout-client';

export const GET: RequestHandler = async (event) => {
	const user = requireUser(event);
	const env = event.platform?.env;
	if (!env?.SYNC_FANOUT || !env.FANOUT_SECRET) throw error(404, 'Fanout not configured');
	if (event.request.headers.get('Upgrade') !== 'websocket') {
		return new Response(null, { status: 204 });
	}
	return proxyAccountWatch(
		{ SYNC_FANOUT: env.SYNC_FANOUT, FANOUT_SECRET: env.FANOUT_SECRET },
		event.request,
		user.id
	);
};
```

In `wrangler.toml`, under the existing `[vars]` comment that mentions `CRON_SECRET`, add `FANOUT_SECRET` to the list of production secrets. Add this binding:

```toml
[[services]]
binding = "SYNC_FANOUT"
service = "eisen-sync-fanout"
```

`FANOUT_SECRET` is a secret. Do not commit a value for it.

- [ ] **Step 4: Run the test to verify it passes**

Run: `npx vitest run src/lib/server/fanout-client.test.ts src/lib/server/sync-notify.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/routes/api/sync/+server.ts src/routes/api/sync/watch/+server.ts src/lib/server/fanout-client.ts src/lib/server/fanout-client.test.ts src/app.d.ts wrangler.toml
git commit -m "$(cat <<'EOF'
Proxy live Sync watch through Pages and notify after an advancing exchange.

EOF
)"
```

---

### Task 5: Fanout Worker

**Files:**
- Create: `workers/sync-fanout/wrangler.toml`
- Create: `workers/sync-fanout/src/index.ts`
- Modify: `package.json` (script `deploy:sync-fanout`)

**Interfaces:**
- Consumes: `changedMessage` and `routeFanoutRequest` from `src/lib/server/account-fanout.ts`
- Produces: Worker `eisen-sync-fanout`, Durable Object class `AccountFanout`, binding `ACCOUNT_FANOUT`. `npm run deploy:sync-fanout` deploys it. The object sends `changedMessage(version)` on `state.getWebSockets()` and accepts a WebSocket with `acceptWebSocket`. It does not read D1.

- [ ] **Step 1: Write the Worker**

`workers/sync-fanout/wrangler.toml`:

```toml
"$schema" = "../../node_modules/wrangler/config-schema.json"
name = "eisen-sync-fanout"
main = "src/index.ts"
compatibility_date = "2025-07-18"

[[durable_objects.bindings]]
name = "ACCOUNT_FANOUT"
class_name = "AccountFanout"

[[migrations]]
tag = "v1"
new_sqlite_classes = ["AccountFanout"]

[secrets]
required = ["FANOUT_SECRET"]
```

`workers/sync-fanout/src/index.ts`:

```ts
import {
	changedMessage,
	routeFanoutRequest
} from '../../../src/lib/server/account-fanout.ts';

interface Env {
	FANOUT_SECRET: string;
	ACCOUNT_FANOUT: DurableObjectNamespace;
}

export class AccountFanout implements DurableObject {
	constructor(private state: DurableObjectState) {}

	async fetch(request: Request): Promise<Response> {
		const url = new URL(request.url);
		if (url.pathname === '/notify' && request.method === 'POST') {
			const body = (await request.json()) as { version?: number };
			if (typeof body.version !== 'number') return new Response('bad notify', { status: 400 });
			const message = changedMessage(body.version);
			for (const socket of this.state.getWebSockets()) socket.send(message);
			return new Response(null, { status: 204 });
		}
		if (request.headers.get('Upgrade') !== 'websocket') {
			return new Response('expected websocket', { status: 426 });
		}
		const pair = new WebSocketPair();
		this.state.acceptWebSocket(pair[1]);
		return new Response(null, { status: 101, webSocket: pair[0] });
	}
}

export default {
	async fetch(request: Request, env: Env): Promise<Response> {
		return routeFanoutRequest(request, {
			secret: env.FANOUT_SECRET,
			stubFor(accountId) {
				return env.ACCOUNT_FANOUT.get(env.ACCOUNT_FANOUT.idFromName(accountId));
			}
		});
	}
} satisfies ExportedHandler<Env>;
```

In `package.json` scripts, add:

```json
"deploy:sync-fanout": "wrangler deploy --config workers/sync-fanout/wrangler.toml"
```

- [ ] **Step 2: Confirm the pure router still passes**

Run: `npx vitest run src/lib/server/account-fanout.test.ts`

Expected: PASS. This task does not add a Vitest that constructs `AccountFanout`. The hub test already covers "notify every socket" and "reject a bad secret". Deploy of the real object is the manual pass in Task 6.

- [ ] **Step 3: Commit**

```bash
git add workers/sync-fanout/wrangler.toml workers/sync-fanout/src/index.ts package.json
git commit -m "$(cat <<'EOF'
Add the sync fanout Worker for visible Workspace listeners.

EOF
)"
```

---

### Task 6: Wire the Workspace

**Files:**
- Modify: `src/lib/workspace/sync-watch.ts` (add `browserSyncWatch`)
- Modify: `src/lib/workspace/workspace.ts`
- Modify: `src/lib/workspace/workspace.test.ts`
- Test: the new case in `src/lib/workspace/workspace.test.ts`

**Interfaces:**
- Consumes: `createSyncWatch`, `SyncWatch`, `SyncWatchHooks`, `WatchSocket` from Task 2. `createAccountFanout` from Task 1. `openWorkspace` adapters from `src/lib/workspace/workspace.ts`.
- Produces:
  - `browserSyncWatch(hooks: SyncWatchHooks): SyncWatch`. When `document` or `WebSocket` is missing, `start` and `stop` do nothing. Otherwise it probes `GET /api/sync/watch` (401, 404, or 204) and then opens `wss`/`ws` `/api/sync/watch`.
  - `WorkspaceAdapters.syncWatch?: (hooks: SyncWatchHooks) => SyncWatch`
  - Workspace starts that watch after the opening Sync attempt settles, whether it succeeded or failed, and stops it on `signOut` and `close`.
  - `sync.lastError` is `{ code: 'session-expired' }` after `onSessionExpired`.

- [ ] **Step 1: Write the failing workspace test**

Add this import and test inside `describe('Workspace')` in `src/lib/workspace/workspace.test.ts`:

```ts
import { createAccountFanout } from '$lib/server/account-fanout';
import { createSyncWatch, type SyncWatchHooks } from './sync-watch';

it('pulls a task on the other device when the fanout pings', async () => {
	const database = memoryMirrorDatabase();
	const mirror = createEncryptedMirror({
		database,
		recoveryObjects: memoryRecoveryObjects(),
		pushDispatch: recordingPushDispatch()
	});
	const inner = cloudPortFor(mirror, ACCOUNT.id);
	const hub = createAccountFanout();
	const cloud = {
		...inner,
		async sync(batch: SyncPushBatch) {
			const before = await database.maxSyncVersion(ACCOUNT.id);
			const result = await inner.sync(batch);
			const after = await database.maxSyncVersion(ACCOUNT.id);
			if (after > before) hub.notify(after);
			return result;
		}
	};
	function syncWatch(hooks: SyncWatchHooks) {
		return createSyncWatch({
			...hooks,
			visibility: { get state() { return 'visible' as const; }, subscribe: () => () => {} },
			online: { get online() { return true; }, subscribe: () => () => {} },
			connect: async () => {
				let onMessage: (data: string) => void = () => {};
				let onClose: () => void = () => {};
				const fanoutSocket = {
					send(data: string) {
						onMessage(data);
					}
				};
				hub.attach(fanoutSocket);
				return {
					ok: true as const,
					socket: {
						onMessage(handler: (data: string) => void) {
							onMessage = handler;
						},
						onClose(handler: () => void) {
							onClose = handler;
						},
						close() {
							hub.detach(fanoutSocket);
							onClose();
						}
					}
				};
			},
			setTimer: () => 1,
			clearTimer: () => {}
		});
	}
	const key = await vaultKeyFromPassword({ password: PASSWORD, cloud, iterations: ITER });
	const a = openWorkspace({
		account: ACCOUNT,
		vaultKey: key,
		adapters: { cloud, clock: clock(), dbName: 'ws-fanout-a', kdfIterations: ITER, syncWatch }
	});
	const b = openWorkspace({
		account: ACCOUNT,
		vaultKey: key,
		adapters: { cloud, clock: clock(5_000), dbName: 'ws-fanout-b', kdfIterations: ITER, syncWatch }
	});
	await a.ready;
	await b.ready;
	await requireOpen(a.state).apply({
		kind: 'create',
		title: 'From A',
		important: true,
		urgent: true
	});
	await vi.waitFor(() => {
		expect(requireOpen(b.state).matrix.cells['do-now'].tasks.map((t) => t.title)).toEqual(['From A']);
	});
	a.close();
	b.close();
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npx vitest run src/lib/workspace/workspace.test.ts`

Expected: FAIL because `WorkspaceAdapters` has no `syncWatch` and B never receives a ping. The type error fails the test file before the assertion.

- [ ] **Step 3: Write the minimal implementation**

Append `browserSyncWatch` to `src/lib/workspace/sync-watch.ts`:

```ts
export function browserSyncWatch(hooks: SyncWatchHooks): SyncWatch {
	if (typeof document === 'undefined' || typeof WebSocket === 'undefined') {
		return { start() {}, stop() {} };
	}
	return createSyncWatch({
		...hooks,
		visibility: {
			get state() {
				return document.visibilityState === 'visible' ? 'visible' : 'hidden';
			},
			subscribe(listener) {
				document.addEventListener('visibilitychange', listener);
				return () => document.removeEventListener('visibilitychange', listener);
			}
		},
		online: {
			get online() {
				return navigator.onLine;
			},
			subscribe(listener) {
				window.addEventListener('online', listener);
				window.addEventListener('offline', listener);
				return () => {
					window.removeEventListener('online', listener);
					window.removeEventListener('offline', listener);
				};
			}
		},
		connect: connectBrowserWatch,
		setTimer(callback, ms) {
			return window.setTimeout(callback, ms);
		},
		clearTimer(id) {
			window.clearTimeout(id);
		}
	});
}

async function connectBrowserWatch(): Promise<WatchConnectResult> {
	let probe: Response;
	try {
		probe = await fetch('/api/sync/watch');
	} catch {
		return { ok: false, status: 0 };
	}
	if (probe.status === 401) return { ok: false, status: 401 };
	if (probe.status === 404) return { ok: false, status: 404 };
	if (probe.status !== 204) return { ok: false, status: 0 };
	const url = new URL('/api/sync/watch', location.href);
	url.protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
	return await new Promise((resolve) => {
		const ws = new WebSocket(url);
		let opened = false;
		ws.addEventListener('open', () => {
			opened = true;
			let onMessage: (data: string) => void = () => {};
			let onClose: () => void = () => {};
			ws.addEventListener('message', (event) => {
				onMessage(typeof event.data === 'string' ? event.data : '');
			});
			ws.addEventListener('close', () => onClose());
			resolve({
				ok: true,
				socket: {
					onMessage(handler) {
						onMessage = handler;
					},
					onClose(handler) {
						onClose = handler;
					},
					close() {
						ws.close();
					}
				}
			});
		});
		const fail = () => {
			if (!opened) resolve({ ok: false, status: 0 });
		};
		ws.addEventListener('error', fail);
		ws.addEventListener('close', fail);
	});
}
```

In `src/lib/workspace/workspace.ts`:

- Import `browserSyncWatch` and `SyncWatchHooks` from `./sync-watch`.
- Add `syncWatch?: (hooks: SyncWatchHooks) => SyncWatch` to `WorkspaceAdapters`. Import type `SyncWatch` too.
- After `syncEngine` is created, add `let sessionExpired = false` and:

```ts
const watch = (opts.adapters.syncWatch ?? browserSyncWatch)({
	runSync: () => syncEngine.run(),
	getLastVersion: () => lastVersion,
	sessionExpired: () => syncEngine.lastError?.code === 'session-expired',
	onSessionExpired: () => {
		sessionExpired = true;
		notify();
	}
});
```

- In the `sync` object inside `snapshotOpen`, change `lastError` to:

```ts
get lastError() {
	if (sessionExpired) return { code: 'session-expired' as const };
	return syncEngine.lastError;
}
```

- At the start of `signOut`, call `watch.stop()`.
- In `close`, call `watch.stop()` before clearing listeners.
- Add `let left = false`. Set `left = true` at the start of `signOut`. Change `ready` so that after `await syncEngine.run()` it does `if (!closed && !left) watch.start()`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `npx vitest run src/lib/workspace/workspace.test.ts src/lib/workspace/sync-watch.test.ts`

Expected: PASS. Existing Workspace tests keep working because Node has no `document`, so `browserSyncWatch` is a no-op.

- [ ] **Step 5: Commit**

```bash
git add src/lib/workspace/sync-watch.ts src/lib/workspace/workspace.ts src/lib/workspace/workspace.test.ts
git commit -m "$(cat <<'EOF'
Start live Sync when the Workspace opens and stop it on sign-out.

EOF
)"
```

- [ ] **Step 6: Manual pass after deploy**

Deploy the Worker with `npm run deploy:sync-fanout`, set the same `FANOUT_SECRET` on that Worker and on the Pages project, then deploy Pages. Open two signed-in browsers on the same Account. Edit a Task in one. The other, left open and visible, shows the Task without Sync now and without a reload. Hide the second tab, edit again in the first, then return: the second tab updates on return. Locking the phone does not sync until the Workspace is visible again.
