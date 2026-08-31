import { error } from '@sveltejs/kit';
import type { RequestEvent } from '@sveltejs/kit';

const COMPARE_BYTES = 256;

function timingSafeEqual(a: string, b: string): boolean {
	const enc = new TextEncoder();
	const left = enc.encode(a);
	const right = enc.encode(b);
	if (left.length > COMPARE_BYTES || right.length > COMPARE_BYTES) return false;
	let result = left.length === right.length ? 0 : 1;
	for (let i = 0; i < COMPARE_BYTES; i++) {
		result |= (left[i] ?? 0) ^ (right[i] ?? 0);
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
