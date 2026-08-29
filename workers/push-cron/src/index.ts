interface Env {
	CRON_SECRET: string;
	TARGET_ORIGIN: string;
}

export default {
	async scheduled(_controller: ScheduledController, env: Env) {
		if (!env.CRON_SECRET || !env.TARGET_ORIGIN) {
			console.error('push-cron: missing CRON_SECRET or TARGET_ORIGIN');
			return;
		}
		const url = new URL('/api/push/cron', env.TARGET_ORIGIN);
		const response = await fetch(url, {
			headers: { Authorization: `Bearer ${env.CRON_SECRET}` }
		});
		if (!response.ok) {
			const body = await response.text();
			console.error(`push-cron: ${response.status} ${body.slice(0, 200)}`);
		}
	}
} satisfies ExportedHandler<Env>;
