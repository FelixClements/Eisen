/// <reference types="@sveltejs/kit" />

import type { Session, User } from 'better-auth/types';

declare global {
	namespace App {
		interface Locals {
			user: User | null;
			session: Session | null;
		}

		interface Platform {
			env: {
				DB: D1Database;
				ATTACHMENTS: R2Bucket;
				APP_NAME: string;
				RECORD_SCHEMA_VERSION: string;
				BETTER_AUTH_SECRET?: string;
				BETTER_AUTH_URL?: string;
				VAPID_PUBLIC_KEY?: string;
				VAPID_PRIVATE_KEY?: string;
				VAPID_SUBJECT?: string;
			};
		}
	}
}

export {};
