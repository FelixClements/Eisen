import { describe, expect, it, vi } from 'vitest';
import { signInTypedOrVerifier } from './account-sign-in';

describe('signInTypedOrVerifier', () => {
	it('uses the verifier when it is accepted', async () => {
		const signIn = vi.fn(async (password: string) =>
			password === 'verifier' ? { error: null } : { error: { message: 'no' } }
		);
		const upgrade = vi.fn();
		await signInTypedOrVerifier({
			typedPassword: 'typed',
			verifier: 'verifier',
			signIn,
			upgrade
		});
		expect(signIn).toHaveBeenCalledOnce();
		expect(signIn).toHaveBeenCalledWith('verifier');
		expect(upgrade).not.toHaveBeenCalled();
	});

	it('falls back to the typed password for pre-verifier accounts', async () => {
		const signIn = vi.fn(async (password: string) =>
			password === 'typed' ? { error: null } : { error: { message: 'Invalid email or password' } }
		);
		const upgrade = vi.fn();
		await signInTypedOrVerifier({
			typedPassword: 'typed',
			verifier: 'verifier',
			signIn,
			upgrade
		});
		expect(signIn).toHaveBeenCalledTimes(2);
		expect(upgrade).toHaveBeenCalledWith('typed', 'verifier');
	});

	it('still succeeds if upgrading the stored password fails', async () => {
		const signIn = vi.fn(async (password: string) =>
			password === 'typed' ? { error: null } : { error: { message: 'Invalid email or password' } }
		);
		await expect(
			signInTypedOrVerifier({
				typedPassword: 'typed',
				verifier: 'verifier',
				signIn,
				upgrade: async () => {
					throw new Error('upgrade failed');
				}
			})
		).resolves.toBeUndefined();
	});

	it('throws the typed-password error when both fail', async () => {
		const signIn = vi.fn(async () => ({ error: { message: 'Invalid email or password' } }));
		await expect(
			signInTypedOrVerifier({
				typedPassword: 'typed',
				verifier: 'verifier',
				signIn
			})
		).rejects.toThrow('Invalid email or password');
	});
});
