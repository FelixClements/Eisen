export type LwwMeta = {
	modifiedAt: number;
	deviceId: string;
};

/** Record-level LWW: greater modifiedAt wins; ties broken by lexicographically greater deviceId. */
export function remoteWins(local: LwwMeta, incoming: LwwMeta): boolean {
	if (incoming.modifiedAt > local.modifiedAt) return true;
	if (incoming.modifiedAt < local.modifiedAt) return false;
	return incoming.deviceId > local.deviceId;
}

export function applyRecordLww(stored: LwwMeta | null, incoming: LwwMeta): 'accept' | 'reject' {
	if (!stored) return 'accept';
	return remoteWins(stored, incoming) ? 'accept' : 'reject';
}
