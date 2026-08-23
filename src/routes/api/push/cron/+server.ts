import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { createEncryptedMirror } from '$lib/server/encrypted-mirror';
import { d1MirrorDatabase } from '$lib/server/adapters/d1';
import { r2RecoveryObjects } from '$lib/server/adapters/r2';
import { noopPushDispatch, webPushDispatch } from '$lib/server/adapters/web-push';

export const GET: RequestHandler = async (event) => {
	const env = event.platform?.env;
	const d1 = env?.DB;
	if (!d1) throw error(500, 'D1 binding not configured');
	const publicKey = env.VAPID_PUBLIC_KEY;
	const privateKey = env.VAPID_PRIVATE_KEY;
	const subject = env.VAPID_SUBJECT ?? 'mailto:admin@eisen.app';
	const mirror = createEncryptedMirror({
		database: d1MirrorDatabase(d1),
		recoveryObjects: env.ATTACHMENTS
			? r2RecoveryObjects(env.ATTACHMENTS)
			: { put: async () => {}, get: async () => null },
		pushDispatch:
			publicKey && privateKey
				? webPushDispatch({ publicKey, privateKey, subject })
				: noopPushDispatch
	});
	const result = await mirror.dispatchDueWakes(Date.now());
	return json(result);
};
