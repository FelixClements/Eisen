import { error } from '@sveltejs/kit';
import type { RequestEvent } from '@sveltejs/kit';

function timingSafeEqual(a: string, b: string): boolean {
	if (a.length !== b.length) return false;
	let result = 0;
	for (let i = 0; i < a.length; i++) {
		result |= a.charCodeAt(i) ^ b.charCodeAt(i);
	}
	return result === 0;
}

export function requireCronSecret(event: RequestEvent): void {
	const secret = event.platform?.env?.CRON_SECRET;
	if (!secret) throw error(503, 'Cron secret not configured.');
	const auth = event.request.headers.get('authorization');
	if (!auth?.startsWith('Bearer ')) throw error(401, 'Unauthorized.');
	const token = auth.slice('Bearer '.length);
	if (!timingSafeEqual(token, secret)) throw error(401, 'Unauthorized.');
}
