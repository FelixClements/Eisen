import { FANOUT_ACCOUNT_HEADER, FANOUT_SECRET_HEADER } from './account-fanout';
import type { FanoutNotify } from './sync-notify';

type FanoutFetcher = {
	fetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response>;
};

export type FanoutWatchBindings = {
	SYNC_FANOUT: FanoutFetcher;
	FANOUT_SECRET: string;
};

export function fanoutWatchBindings(
	env: App.Platform['env'] | undefined
): FanoutWatchBindings | null {
	if (!env?.SYNC_FANOUT || !env.FANOUT_SECRET) return null;
	return { SYNC_FANOUT: env.SYNC_FANOUT, FANOUT_SECRET: env.FANOUT_SECRET };
}

export function fanoutFromEnv(env: App.Platform['env'] | undefined): FanoutNotify | null {
	const bindings = fanoutWatchBindings(env);
	if (!bindings) return null;
	const binding = bindings.SYNC_FANOUT;
	const secret = bindings.FANOUT_SECRET;
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
	headers.delete('Cookie');
	headers.delete('Authorization');
	headers.set(FANOUT_SECRET_HEADER, env.FANOUT_SECRET);
	headers.set(FANOUT_ACCOUNT_HEADER, accountId);
	return env.SYNC_FANOUT.fetch(new Request(request, { headers }));
}
