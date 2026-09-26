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
			// Simulate sync advancing local version so retry logic does not double-count.
			version = Math.max(version, 2);
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
		const box: { current: ((data: string) => void) | null } = { current: null };
		const watch = createSyncWatch({
			visibility: { get state() { return 'visible' as const; }, subscribe: () => () => {} },
			online: { get online() { return true; }, subscribe: () => () => {} },
			connect: async () => ({
				ok: true,
				socket: {
					onMessage(handler) {
						box.current = handler;
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
		box.current?.(JSON.stringify({ type: 'changed', version: 4 }));
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
		const box: { current: ((data: string) => void) | null } = { current: null };
		const watch = createSyncWatch({
			visibility: { get state() { return 'visible' as const; }, subscribe: () => () => {} },
			online: { get online() { return true; }, subscribe: () => () => {} },
			connect: async () => ({
				ok: true,
				socket: {
					onMessage(handler) {
						box.current = handler;
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
		box.current?.(JSON.stringify({ type: 'changed', version: 5 }));
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
		const box: { current: ((data: string) => void) | null } = { current: null };
		const watch = createSyncWatch({
			visibility: { get state() { return 'visible' as const; }, subscribe: () => () => {} },
			online: { get online() { return true; }, subscribe: () => () => {} },
			connect: async () => ({
				ok: true,
				socket: {
					onMessage(handler) {
						box.current = handler;
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
		box.current?.(JSON.stringify({ type: 'changed', version: 9 }));
		await Promise.resolve();
		await Promise.resolve();
		expect(expired).toBe(1);
		expect(timers).toEqual([]);
	});

	it('does not run follow-up sync after hiding during in-flight pump sync', async () => {
		let version = 1;
		let calls = 0;
		let release: () => void = () => {};
		const gate = new Promise<void>((resolve) => {
			release = resolve;
		});
		let visibility: 'visible' | 'hidden' = 'visible';
		const visListeners = new Set<() => void>();
		const box: { current: ((data: string) => void) | null } = { current: null };
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
			online: { get online() { return true; }, subscribe: () => () => {} },
			connect: async () => ({
				ok: true,
				socket: {
					onMessage(handler) {
						box.current = handler;
					},
					onClose() {},
					close() {}
				}
			}),
			runSync: async () => {
				calls += 1;
				if (calls === 1) {
					await gate;
				}
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
		box.current?.(JSON.stringify({ type: 'changed', version: 5 }));
		await Promise.resolve();
		expect(calls).toBe(1);
		visibility = 'hidden';
		for (const listener of visListeners) listener();
		release();
		await gate;
		await Promise.resolve();
		await Promise.resolve();
		expect(calls).toBe(1);
	});

	it('closes the previous socket on 401 so reconnect is not armed', async () => {
		let failConnect = false;
		let expired = 0;
		const timerMs: number[] = [];
		const onlineListeners = new Set<() => void>();
		let watchOnClose: (() => void) | undefined;
		let closeCalls = 0;
		const firstSocket: WatchSocket = {
			onMessage() {},
			onClose(handler) {
				watchOnClose = handler;
			},
			close() {
				closeCalls += 1;
				watchOnClose?.();
			}
		};
		const watch = createSyncWatch({
			visibility: { get state() { return 'visible' as const; }, subscribe: () => () => {} },
			online: {
				get online() { return true; },
				subscribe(listener) {
					onlineListeners.add(listener);
					return () => onlineListeners.delete(listener);
				}
			},
			connect: async () => {
				if (!failConnect) {
					failConnect = true;
					return { ok: true, socket: firstSocket };
				}
				return { ok: false, status: 401 };
			},
			runSync: async () => {},
			getLastVersion: () => 1,
			sessionExpired: () => false,
			onSessionExpired: () => {
				expired += 1;
			},
			setTimer: (_fn, ms) => {
				timerMs.push(ms);
				return 1;
			},
			clearTimer: () => {}
		});
		watch.start();
		await Promise.resolve();
		await Promise.resolve();
		for (const listener of onlineListeners) listener();
		await Promise.resolve();
		await Promise.resolve();
		expect(expired).toBe(1);
		expect(closeCalls).toBe(1);
		watchOnClose?.();
		expect(timerMs).toEqual([]);
	});

	it('closes the previous socket on 404 so reconnect is not armed', async () => {
		let failConnect = false;
		const timerMs: number[] = [];
		const onlineListeners = new Set<() => void>();
		let watchOnClose: (() => void) | undefined;
		let closeCalls = 0;
		const firstSocket: WatchSocket = {
			onMessage() {},
			onClose(handler) {
				watchOnClose = handler;
			},
			close() {
				closeCalls += 1;
				watchOnClose?.();
			}
		};
		const watch = createSyncWatch({
			visibility: { get state() { return 'visible' as const; }, subscribe: () => () => {} },
			online: {
				get online() { return true; },
				subscribe(listener) {
					onlineListeners.add(listener);
					return () => onlineListeners.delete(listener);
				}
			},
			connect: async () => {
				if (!failConnect) {
					failConnect = true;
					return { ok: true, socket: firstSocket };
				}
				return { ok: false, status: 404 };
			},
			runSync: async () => {},
			getLastVersion: () => 1,
			sessionExpired: () => false,
			onSessionExpired: () => {},
			setTimer: (_fn, ms) => {
				timerMs.push(ms);
				return 1;
			},
			clearTimer: () => {}
		});
		watch.start();
		await Promise.resolve();
		await Promise.resolve();
		for (const listener of onlineListeners) listener();
		await Promise.resolve();
		await Promise.resolve();
		expect(closeCalls).toBe(1);
		watchOnClose?.();
		expect(timerMs).toEqual([]);
	});
});
