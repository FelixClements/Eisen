/// <reference types="@sveltejs/kit" />
/// <reference types="./worker-configuration" />

declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}

		interface Platform {
			env: {
				DB: D1Database;
				KV: KVNamespace;
				ATTACHMENTS: R2Bucket;
				APP_NAME: 'Eisen';
				RECORD_SCHEMA_VERSION: '1';
			};
		}
	}
}

export {};
