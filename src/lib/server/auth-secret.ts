export function requireAuthSecret(secret: string | undefined): string {
	if (!secret) throw new Error('BETTER_AUTH_SECRET is not configured');
	return secret;
}

export function requireAuthUrl(url: string | undefined): string {
	if (!url) throw new Error('BETTER_AUTH_URL is not configured');
	return url;
}
