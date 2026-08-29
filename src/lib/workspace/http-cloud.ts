import { EisenErrorException, type BackupRef } from './types';
import type { CloudPort, SyncPushBatch, VaultParams } from './ports';

function mapStatus(status: number): never {
	if (status === 401) throw new EisenErrorException({ code: 'session-expired' });
	throw new EisenErrorException({ code: 'server', status });
}

async function parseJson<T>(response: Response): Promise<T> {
	if (response.status === 404) throw new EisenErrorException({ code: 'server', status: 404 });
	if (!response.ok) mapStatus(response.status);
	return response.json() as Promise<T>;
}

export function httpCloud(fetchFn: typeof fetch = fetch): CloudPort {
	return {
		async getVaultParams() {
			const response = await fetchFn('/api/vault-params');
			if (response.status === 404) return null;
			if (!response.ok) mapStatus(response.status);
			return response.json() as Promise<VaultParams>;
		},
		async createVaultParams(params) {
			const response = await fetchFn('/api/vault-params', {
				method: 'PUT',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(params)
			});
			if (response.status === 409) throw new EisenErrorException({ code: 'vault-exists' });
			if (!response.ok) mapStatus(response.status);
		},
		async sync(batch: SyncPushBatch) {
			let response: Response;
			try {
				response = await fetchFn('/api/sync', {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify(batch)
				});
			} catch {
				throw new EisenErrorException({ code: 'offline' });
			}
			return parseJson(response);
		},
		async putBackup(pkg) {
			const response = await fetchFn('/api/backup', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ packageId: pkg.id, packageText: pkg.text, deviceId: pkg.deviceId })
			});
			const data = await parseJson<{ packageId: string }>(response);
			return { id: data.packageId, createdAt: Date.now() } satisfies BackupRef;
		},
		async listBackups() {
			const response = await fetchFn('/api/backup');
			const data = await parseJson<{ backups: { packageId: string; createdAt: number }[] }>(response);
			return data.backups.map((b) => ({ id: b.packageId, createdAt: b.createdAt }));
		},
		async getBackup(packageId) {
			const response = await fetchFn(`/api/backup/${packageId}`);
			if (!response.ok) mapStatus(response.status);
			return response.text();
		},
		async scheduleWake(w) {
			const response = await fetchFn('/api/push/schedule', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(w)
			});
			if (!response.ok) mapStatus(response.status);
		},
		async registerPush(sub) {
			const response = await fetchFn('/api/push/subscribe', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(sub)
			});
			if (!response.ok) mapStatus(response.status);
		},
		async sendTestPush(deviceId) {
			const response = await fetchFn('/api/push/test', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ deviceId })
			});
			if (response.status === 404) throw new EisenErrorException({ code: 'server', status: 404 });
			if (response.status === 502) throw new EisenErrorException({ code: 'server', status: 502 });
			if (!response.ok) mapStatus(response.status);
		}
	};
}
