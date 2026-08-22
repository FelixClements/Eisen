import type { RecoveryObjectPort } from '../encrypted-mirror';

export function r2RecoveryObjects(bucket: R2Bucket): RecoveryObjectPort {
	return {
		async put(key, body) {
			await bucket.put(key, body, {
				httpMetadata: { contentType: 'application/eisen-recovery' }
			});
		},
		async get(key) {
			const obj = await bucket.get(key);
			if (!obj) return null;
			return obj.text();
		}
	};
}
