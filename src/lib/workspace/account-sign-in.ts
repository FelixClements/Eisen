export type AuthAttemptError = { message?: string | null } | null;

export async function signInTypedOrVerifier(opts: {
	typedPassword: string;
	verifier: string;
	signIn: (password: string) => Promise<{ error: AuthAttemptError }>;
	upgrade?: (currentPassword: string, newPassword: string) => Promise<void>;
}): Promise<void> {
	const withVerifier = await opts.signIn(opts.verifier);
	if (!withVerifier.error) return;
	const withTyped = await opts.signIn(opts.typedPassword);
	if (withTyped.error) {
		throw new Error(withTyped.error.message ?? withVerifier.error.message ?? 'Sign in failed');
	}
	try {
		await opts.upgrade?.(opts.typedPassword, opts.verifier);
	} catch {
		// Signed in; upgrading the stored hash is best-effort.
	}
}
