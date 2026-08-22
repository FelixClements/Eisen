import type { PushDispatchPort, PushSubscriptionRow } from '../encrypted-mirror';
// @ts-expect-error web-push types may be missing in dev
import webpush from 'web-push';

export function webPushDispatch(opts: {
	publicKey: string;
	privateKey: string;
	subject: string;
}): PushDispatchPort {
	webpush.setVapidDetails(opts.subject, opts.publicKey, opts.privateKey);
	return {
		async send(sub: PushSubscriptionRow, payload: string) {
			await webpush.sendNotification(
				{
					endpoint: sub.endpoint,
					keys: { p256dh: sub.p256dh, auth: sub.auth }
				},
				payload
			);
		}
	};
}

export const noopPushDispatch: PushDispatchPort = {
	async send() {
		/* VAPID unset */
	}
};
