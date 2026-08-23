import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { requireUser } from '$lib/server/require-user';
import { mirrorFromEvent } from '$lib/server/mirror-from-event';
import { VaultParamsExistError } from '$lib/server/encrypted-mirror';

export const GET: RequestHandler = async (event) => {
	const user = requireUser(event);
	const mirror = mirrorFromEvent(event);
	const params = await mirror.getVaultParams(user.id);
	if (!params) throw error(404, 'No vault params');
	return json(params);
};

export const PUT: RequestHandler = async (event) => {
	const user = requireUser(event);
	const mirror = mirrorFromEvent(event);
	const body = (await event.request.json()) as { salt?: string; checkBlob?: string };
	if (!body.salt || !body.checkBlob) throw error(400, 'Missing vault params.');
	try {
		await mirror.createVaultParams(user.id, { salt: body.salt, checkBlob: body.checkBlob });
	} catch (e) {
		if (e instanceof VaultParamsExistError) throw error(409, 'Vault params exist');
		throw e;
	}
	return json({ ok: true });
};
