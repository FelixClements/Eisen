import { error } from '@sveltejs/kit';
import type { RequestEvent } from '@sveltejs/kit';
import { createEncryptedMirror } from './encrypted-mirror';
import { d1MirrorDatabase } from './adapters/d1';
import { r2RecoveryObjects } from './adapters/r2';
import { noopPushDispatch, webPushDispatch } from './adapters/web-push';

export function mirrorFromEvent(event: RequestEvent) {
	const env = event.platform?.env;
	if (!env?.DB) throw error(500, 'D1 binding not configured');
	const publicKey = env.VAPID_PUBLIC_KEY;
	const privateKey = env.VAPID_PRIVATE_KEY;
	const subject = env.VAPID_SUBJECT ?? 'mailto:admin@eisen.app';
	return createEncryptedMirror({
		database: d1MirrorDatabase(env.DB),
		recoveryObjects: env.ATTACHMENTS
			? r2RecoveryObjects(env.ATTACHMENTS)
			: { put: async () => {}, get: async () => null },
		pushDispatch:
			publicKey && privateKey
				? webPushDispatch({ publicKey, privateKey, subject })
				: noopPushDispatch
	});
}
