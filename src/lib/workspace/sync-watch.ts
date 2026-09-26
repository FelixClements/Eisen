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
