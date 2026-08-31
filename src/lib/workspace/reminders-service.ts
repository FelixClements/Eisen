import { EisenErrorException } from './types';
import type {
	Clock,
	ReminderEnableResult,
	ReminderTestResult,
	RemindersPort,
	WakePort
} from './ports';
import type { CatalogTask } from './task-codec';
import { clearReminderCache, syncReminderCache } from '$lib/reminder-cache';

export type RemindersService = {
	permission(): 'unsupported' | 'default' | 'granted' | 'denied';
	enable(): Promise<ReminderEnableResult>;
	reattachIfGranted(): Promise<void>;
	testPush(): Promise<ReminderTestResult>;
	dueReminders(): { id: string; title: string }[];
	scheduleWake(): Promise<void>;
	refreshCache(): Promise<void>;
	onCatalogChanged(): void;
	onSignOut(): Promise<void>;
	get lastScheduleError(): string | null;
};

export function createRemindersService(opts: {
	wake: WakePort;
	reminders: RemindersPort;
	clock: Clock;
	getDeviceId: () => string;
	getTasks: () => Iterable<CatalogTask>;
	isOpen: () => boolean;
}): RemindersService {
	let pushEnabled = false;
	let lastScheduleError: string | null = null;

	function activeReminders() {
		return [...opts.getTasks()].filter(
			(t) => !t.deleted && !t.completed && !t.archived && t.remindAt
		);
	}

	function upcoming(now = opts.clock.now()) {
		return activeReminders()
			.filter((t) => (t.remindAt as number) > now)
			.sort((a, b) => (a.remindAt as number) - (b.remindAt as number))
			.map((t) => ({ id: t.id, title: t.title, remindAt: t.remindAt as number }));
	}

	async function refreshCache() {
		if (!pushEnabled) return;
		try {
			await syncReminderCache(upcoming());
		} catch (err) {
			console.error('reminder cache sync failed:', err);
		}
	}

	async function scheduleWake() {
		const next = upcoming()[0];
		if (!next) return;
		try {
			await opts.wake.scheduleWake({
				deviceId: opts.getDeviceId(),
				wakeAt: next.remindAt,
				nonce: opts.clock.uuid()
			});
			lastScheduleError = null;
		} catch (err) {
			lastScheduleError = err instanceof Error ? err.message : 'scheduleWake failed';
			console.error('scheduleWake failed:', err);
		}
	}

	async function registerCurrentSubscription(): Promise<ReminderEnableResult> {
		if (!opts.reminders.vapidReady()) return 'vapid-missing';
		const sub = await opts.reminders.subscribe();
		if (!sub) return 'subscribe-failed';
		try {
			await opts.wake.registerPush({ deviceId: opts.getDeviceId(), ...sub });
		} catch (err) {
			console.error('registerPush failed:', err);
			return 'server-error';
		}
		pushEnabled = true;
		await refreshCache();
		await scheduleWake();
		return 'granted';
	}

	async function enable(): Promise<ReminderEnableResult> {
		const initial = opts.reminders.permission();
		if (initial === 'unsupported') return 'unsupported';
		const perm = await opts.reminders.request();
		if (perm === 'unsupported') return 'unsupported';
		if (perm === 'denied') return 'denied';
		return registerCurrentSubscription();
	}

	async function reattachIfGranted(): Promise<void> {
		if (opts.reminders.permission() !== 'granted') return;
		await registerCurrentSubscription();
	}

	async function testPush(): Promise<ReminderTestResult> {
		if (opts.reminders.permission() !== 'granted') return 'permission-denied';
		try {
			await opts.wake.sendTestPush(opts.getDeviceId());
			return 'sent';
		} catch (err) {
			if (err instanceof EisenErrorException && err.error.code === 'server') {
				if (err.error.status === 404) return 'no-subscription';
				if (err.error.status === 502 || err.error.status === 503) return 'push-failed';
			}
			console.error('sendTestPush failed:', err);
			return 'server-error';
		}
	}

	return {
		permission: () => opts.reminders.permission(),
		enable,
		reattachIfGranted,
		testPush,
		dueReminders() {
			const now = opts.clock.now();
			return activeReminders()
				.filter((t) => (t.remindAt as number) <= now)
				.map((t) => ({ id: t.id, title: t.title }));
		},
		scheduleWake,
		refreshCache,
		onCatalogChanged() {
			if (!opts.isOpen()) return;
			void refreshCache();
			void scheduleWake();
		},
		async onSignOut() {
			try {
				await opts.reminders.unsubscribe();
			} catch (err) {
				console.error('push unsubscribe failed:', err);
			}
			try {
				await opts.wake.unregisterPush({ deviceId: opts.getDeviceId() });
			} catch (err) {
				console.error('unregisterPush failed:', err);
			}
			await clearReminderCache();
			pushEnabled = false;
		},
		get lastScheduleError() {
			return lastScheduleError;
		}
	};
}
