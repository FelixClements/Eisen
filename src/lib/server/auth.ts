/// <reference types="@cloudflare/workers-types" />

import { betterAuth, type BetterAuthOptions } from 'better-auth';
import { sveltekitCookies } from 'better-auth/svelte-kit';
import { getRequestEvent } from '$app/server';
import { Kysely } from 'kysely';
import { D1Dialect } from 'kysely-d1';

export function createAuth(d1: D1Database, secret: string, baseURL: string) {
	const db = new Kysely({ dialect: new D1Dialect({ database: d1 }) });

	const options = {
		database: {
			db,
			type: 'sqlite'
		},
		secret,
		baseURL,
		telemetry: { enabled: false },
		emailAndPassword: {
			enabled: true
		},
		plugins: [sveltekitCookies(getRequestEvent)]
	} satisfies BetterAuthOptions;

	return betterAuth(options);
}

export type Auth = ReturnType<typeof createAuth>;
