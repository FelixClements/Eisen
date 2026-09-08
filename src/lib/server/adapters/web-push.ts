import { buildPushHTTPRequest } from '@pushforge/builder';
import type { PushDispatchPort, PushSubscriptionRow } from '../encrypted-mirror';

export class PushSendError extends Error {
	readonly status: number;

	constructor(message: string, status: number) {
		super(message);
		this.name = 'PushSendError';
		this.status = status;
	}
}

export function webPushDispatch(opts: {
	privateKeyJwk: string;
	subject: string;
}): PushDispatchPort {
	let privateJWK: string | JsonWebKey = opts.privateKeyJwk;
	try {
		privateJWK = JSON.parse(opts.privateKeyJwk) as JsonWebKey;
	} catch {
		privateJWK = opts.privateKeyJwk;
	}

	return {
		async send(sub: PushSubscriptionRow, payload: string) {
			const { endpoint, headers, body } = await buildPushHTTPRequest({
				privateJWK,
				subscription: {
					endpoint: sub.endpoint,
					keys: { p256dh: sub.p256dh, auth: sub.auth }
				},
				message: {
					payload: JSON.parse(payload) as { type: string },
					adminContact: opts.subject,
					// FCM 403s JWTs whose exp is exactly 24h once clocks differ by a second.
					options: { ttl: 12 * 60 * 60 }
				}
			});

			const response = await fetch(endpoint, { method: 'POST', headers, body });
			if (response.ok) return;

			const host = new URL(sub.endpoint).host;
			const detail = (await response.text()).slice(0, 180).replace(/\s+/g, ' ');
			console.error(`push send failed host=${host} status=${response.status} body=${detail}`);
			throw new PushSendError(`Push send failed for ${host}: ${response.status}`, response.status);
		}
	};
}

export function unconfiguredPushDispatch(): PushDispatchPort {
	return {
		async send() {
			throw new PushSendError('VAPID unset', 503);
		}
	};
}
