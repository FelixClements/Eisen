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
