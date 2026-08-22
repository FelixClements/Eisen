/** Record-level LWW: greater updatedAt wins; ties broken by lexicographically greater deviceId. */
export function remoteWins(local: {
	updatedAt: number;
	deviceId: string;
}, incoming: { updatedAt: number; deviceId: string }): boolean {
	if (incoming.updatedAt > local.updatedAt) return true;
	if (incoming.updatedAt < local.updatedAt) return false;
	return incoming.deviceId > local.deviceId;
}
