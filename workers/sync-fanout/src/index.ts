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
			for (const socket of this.state.getWebSockets()) {
				try {
					socket.send(message);
				} catch {
					// skip dead sockets
				}
			}
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
