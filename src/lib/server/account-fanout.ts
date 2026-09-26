export const FANOUT_SECRET_HEADER = 'X-Fanout-Secret';
export const FANOUT_ACCOUNT_HEADER = 'X-Account-Id';

export function changedMessage(version: number): string {
	return JSON.stringify({ type: 'changed', version });
}

export function authorizeFanout(header: string | null, secret: string | undefined): boolean {
	return !!secret && secret.length > 0 && header === secret;
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
			for (const socket of sockets) {
				try {
					socket.send(message);
				} catch {
					// skip dead sockets
				}
			}
		}
	};
}

export type FanoutStub = {
	fetch(request: Request): Promise<Response>;
};

export async function routeFanoutRequest(
	request: Request,
	opts: { secret: string | undefined; stubFor(accountId: string): FanoutStub }
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
