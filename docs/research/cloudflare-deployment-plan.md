# Eisen SvelteKit PWA Deployment Guide to Cloudflare Pages

This guide covers deploying the Eisen SvelteKit PWA to Cloudflare Pages with D1, KV, and R2 bindings.

## 1. Exact Prerequisites

### Required Accounts and Tools
- **Cloudflare Account**: You need a Cloudflare account with access to Workers & Pages (https://developers.cloudflare.com/workers/wrangler/install-and-update/)
- **Node.js**: Wrangler supports Current, Active, and Maintenance versions of Node.js (https://developers.cloudflare.com/workers/wrangler/install-and-update/). The project uses Node.js for the build process.
- **Wrangler CLI**: Version 3.45.0 or higher required for Pages Wrangler configuration support (https://developers.cloudflare.com/pages/functions/wrangler-configuration/). The project has `wrangler@^3.91.0` in package.json (/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/package.json:25).

### Authentication
Run `wrangler login` to authenticate with your Cloudflare account via OAuth (https://developers.cloudflare.com/workers/wrangler/commands/general/):

```bash
npx wrangler login
```

This will open a browser for OAuth authentication. For CI/CD environments, you can use API tokens instead.

## 2. How @sveltejs/adapter-cloudflare Works

### Adapter Functionality
The `@sveltejs/adapter-cloudflare` adapter builds SvelteKit applications for Cloudflare Workers Static Assets or Cloudflare Pages with Workers integration (https://svelte.dev/docs/kit/adapter-cloudflare). It:
- Builds the application to `.svelte-kit/cloudflare` directory
- Emulates `event.platform` during local development
- Automatically applies type declarations for Cloudflare bindings
- Generates the necessary Worker/Pages configuration

### Expected wrangler.toml Structure
For Cloudflare Pages, the adapter expects a wrangler.toml with (https://svelte.dev/docs/kit/adapter-cloudflare):
- `pages_build_output_dir`: Points to `.svelte-kit/cloudflare` (Pages-specific)
- `compatibility_date`: Runtime compatibility date
- `compatibility_flags`: Runtime flags like `nodejs_als` for AsyncLocalStorage
- Binding configurations for D1, KV, R2

The current wrangler.toml (/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/wrangler.toml) is correctly configured:

```toml
pages_build_output_dir = ".svelte-kit/cloudflare"
compatibility_date = "2025-07-18"
compatibility_flags = ["nodejs_als"]
```

### Build Output
The adapter outputs to `.svelte-kit/cloudflare` which is the correct build directory for SvelteKit on Pages (https://developers.cloudflare.com/pages/configuration/build-configuration/).

## 3. Creating Real D1, KV, and R2 Resources

### D1 Database
Create a D1 database and get its ID (https://developers.cloudflare.com/d1/get-started/):

```bash
npx wrangler d1 create eisen-db
```

This command:
- Creates a new D1 database named "eisen-db"
- Outputs the `database_id` (UUID)
- Prompts to automatically add the binding to wrangler.toml with `--update-config` flag

The output will include:

```toml
[[d1_databases]]
binding = "DB"
database_name = "eisen-db"
database_id = "<UUID>"
```

### KV Namespace
Create a KV namespace (https://developers.cloudflare.com/kv/get-started/):

```bash
npx wrangler kv namespace create KV
```

This outputs:

```toml
[[kv_namespaces]]
binding = "KV"
id = "<NAMESPACE_ID>"
```

### R2 Bucket
Create an R2 bucket (https://developers.cloudflare.com/r2/get-started/cli/):

```bash
npx wrangler r2 bucket create eisen-attachments
```

Note: R2 buckets don't have a binding ID like D1/KV. The binding uses the bucket name directly in wrangler.toml (https://developers.cloudflare.com/r2/reference/wrangler-commands/).

## 4. Updating wrangler.toml with Real IDs

### Replace Placeholder IDs
Update the wrangler.toml (/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/wrangler.toml) with the real IDs from the creation commands:

```toml
[[kv_namespaces]]
binding = "KV"
id = "YOUR_REAL_KV_NAMESPACE_ID"  # Replace from wrangler kv namespace create output

[[d1_databases]]
binding = "DB"
database_name = "eisen-db"
database_id = "YOUR_REAL_D1_DATABASE_ID"  # Replace from wrangler d1 create output
# Add preview_database_id for local development
preview_database_id = "local-eisen-db"

[[r2_buckets]]
binding = "ATTACHMENTS"
bucket_name = "eisen-attachments"  # This is the bucket name, not an ID

[vars]
APP_NAME = "Eisen"
RECORD_SCHEMA_VERSION = "1"
```

### Environment-Specific Configuration
For different environments (preview vs production), use environment overrides (https://developers.cloudflare.com/pages/functions/wrangler-configuration/):

```toml
[vars]
APP_NAME = "Eisen"
RECORD_SCHEMA_VERSION = "1"

[env.production.vars]
APP_NAME = "Eisen Production"

[env.preview.vars]
APP_NAME = "Eisen Preview"
```

## 5. Applying D1 Migrations

### Local Development (Default)
By default, `wrangler d1 migrations apply` targets the local database (https://developers.cloudflare.com/d1/best-practices/local-development/):

```bash
npx wrangler d1 migrations apply eisen-db --local
```

This applies migrations to the local database defined by `preview_database_id`.

### Production (Remote)
To apply migrations to the production D1 database, use the `--remote` flag (https://developers.cloudflare.com/d1/reference/migrations/):

```bash
npx wrangler d1 migrations apply eisen-db --remote
```

**Important**: As of Wrangler v3, migrations default to local. You must explicitly use `--remote` for production (https://github.com/cloudflare/workers-sdk/pull/4930).

### Migration Files
The project has two migration files:
- `/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/migrations/0001_init.sql` - Creates vault_records table
- `/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/migrations/0002_add_accounts_and_devices.sql` - Creates accounts, devices, and backups tables

These will be applied in order by the migrations system.

### Preview Database for Local Development
For Pages local development with D1, you need a `preview_database_id` in wrangler.toml (https://developers.cloudflare.com/d1/best-practices/local-development/):

```toml
[[d1_databases]]
binding = "DB"
database_name = "eisen-db"
database_id = "<PRODUCTION_UUID>"
preview_database_id = "local-eisen-db"  # For local development
```

## 6. Exact CLI Commands to Deploy

### Build the Application

```bash
npm run build
```

This runs `vite build` and outputs to `.svelte-kit/cloudflare` (https://svelte.dev/docs/kit/adapter-cloudflare).

### Create Pages Project (First Time Only)

```bash
npx wrangler pages project create eisen-svelte-pwa
```

This creates a new Pages project (https://developers.cloudflare.com/pages/get-started/direct-upload/).

### Deploy to Production

```bash
npx wrangler pages deploy .svelte-kit/cloudflare
```

This deploys the built assets to production (https://developers.cloudflare.com/workers/wrangler/commands/pages/).

### Deploy to Preview

```bash
npx wrangler pages deploy .svelte-kit/cloudflare --branch=feature-branch
```

This creates a preview deployment (https://developers.cloudflare.com/pages/get-started/direct-upload/).

### Local Development

```bash
npm run preview
```

This runs `wrangler pages dev .svelte-kit/cloudflare` as defined in package.json (/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/package.json:8).

### Apply Migrations Before Deployment

```bash
# For production
npx wrangler d1 migrations apply eisen-db --remote

# For local
npx wrangler d1 migrations apply eisen-db --local
```

## 7. Environment Variables and Secrets Handling

### Environment Variables in wrangler.toml
Non-sensitive variables are defined in the `[vars]` section (https://developers.cloudflare.com/pages/functions/bindings/):

```toml
[vars]
APP_NAME = "Eisen"
RECORD_SCHEMA_VERSION = "1"
```

These are accessible in your code via `platform.env.APP_NAME` (https://svelte.dev/docs/kit/adapter-cloudflare).

### Secrets (Sensitive Values)
For sensitive values like API keys, use `wrangler pages secret put` (https://developers.cloudflare.com/workers/wrangler/commands/pages/):

```bash
npx wrangler pages secret put API_KEY --env=production
npx wrangler pages secret put API_KEY --env=preview
```

Secrets are encrypted and only accessible programmatically via `context.env` (https://developers.cloudflare.com/pages/functions/bindings/).

### Accessing Variables in Code
The app uses TypeScript definitions in `/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/src/app.d.ts`:

```typescript
interface Platform {
    env: {
        DB: D1Database;
        KV: KVNamespace;
        ATTACHMENTS: R2Bucket;
        APP_NAME: 'Eisen';
        RECORD_SCHEMA_VERSION: '1';
    };
}
```

Access in API routes via `platform.env` (e.g., `/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/src/routes/api/sync/+server.ts:19`):

```typescript
const d1 = platform?.env?.DB;
const kv = platform?.env?.KV;
const r2 = platform?.env?.ATTACHMENTS;
```

## 8. Preview vs Production Deployments

### Key Differences
- **Production**: Deployed to the main branch, available at `project.pages.dev` (https://developers.cloudflare.com/pages/configuration/preview-deployments/)
- **Preview**: Deployed to feature branches, available at `hash.project.pages.dev` and `branchname.project.pages.dev`
- **Configuration**: Use `[env.production]` and `[env.preview]` sections in wrangler.toml for environment-specific settings (https://developers.cloudflare.com/pages/functions/wrangler-configuration/)

### Preview Deployment Behavior
- Each branch gets a unique hash-based URL
- Branch-based alias updates as you push commits
- Custom domains are not affected by preview deployments
- Can be protected with Cloudflare Access (https://developers.cloudflare.com/pages/configuration/preview-deployments/)

### Local Development (wrangler pages dev)
- Serves static assets and runs Functions locally
- Uses bindings from wrangler.toml by default
- Can override bindings with CLI flags (e.g., `--d1`, `--kv`) (https://developers.cloudflare.com/pages/functions/local-development/)
- Default port is 8788 (as configured in playwright.config.ts:12)

## 9. Step-by-Step Deployment Checklist

### Initial Setup (One-Time)
1. **Install dependencies**: `npm install`
2. **Authenticate with Cloudflare**: `npx wrangler login`
3. **Create D1 database**: `npx wrangler d1 create eisen-db`
4. **Create KV namespace**: `npx wrangler kv namespace create KV`
5. **Create R2 bucket**: `npx wrangler r2 bucket create eisen-attachments`
6. **Update wrangler.toml**: Replace placeholder IDs with real IDs from steps 3-5
7. **Add preview_database_id**: Add `preview_database_id = "local-eisen-db"` to D1 binding
8. **Create Pages project**: `npx wrangler pages project create eisen-svelte-pwa`

### Local Development
1. **Apply local migrations**: `npx wrangler d1 migrations apply eisen-db --local`
2. **Build application**: `npm run build`
3. **Start local server**: `npm run preview`
4. **Test locally**: Access at `http://localhost:8788`

### Production Deployment
1. **Apply production migrations**: `npx wrangler d1 migrations apply eisen-db --remote`
2. **Build application**: `npm run build`
3. **Deploy to production**: `npx wrangler pages deploy .svelte-kit/cloudflare`
4. **Verify deployment**: Check the deployment URL in Cloudflare dashboard

### Preview Deployment
1. **Apply migrations** (optional, can use production DB): `npx wrangler d1 migrations apply eisen-db --remote`
2. **Build application**: `npm run build`
3. **Deploy to preview**: `npx wrangler pages deploy .svelte-kit/cloudflare --branch=your-branch`
4. **Verify preview**: Access at the hash-based URL

### Adding Secrets
1. **Add production secret**: `npx wrangler pages secret put SECRET_NAME --env=production`
2. **Add preview secret**: `npx wrangler pages secret put SECRET_NAME --env=preview`
3. **Redeploy**: Secrets require a new deployment to take effect

## 10. Common Gotchas and Verification

### Common Gotchas

1. **Migration Direction**: `wrangler d1 migrations apply` now defaults to `--local`. You must explicitly use `--remote` for production (https://github.com/cloudflare/workers-sdk/pull/4930).

2. **Missing preview_database_id**: For Pages local development with D1, you must have `preview_database_id` set in wrangler.toml (https://developers.cloudflare.com/d1/best-practices/local-development/).

3. **Binding Name Mismatches**: The binding name in wrangler.toml must match the TypeScript definition in app.d.ts. The app uses `DB`, `KV`, and `ATTACHMENTS` (/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/src/app.d.ts:12-14).

4. **Node.js Compatibility**: The app uses `nodejs_als` compatibility flag for AsyncLocalStorage. Ensure your compatibility_date is recent enough (2025-07-18 in current config) (https://developers.cloudflare.com/workers/configuration/compatibility-flags/).

5. **R2 Bucket Names**: R2 uses bucket names, not IDs. The binding in wrangler.toml references the bucket name directly (https://developers.cloudflare.com/r2/reference/wrangler-commands/).

6. **Functions Directory**: SvelteKit server endpoints are used instead of a `/functions` directory. Functions in `/functions` at project root will not be included (https://svelte.dev/docs/kit/adapter-cloudflare).

7. **Build Directory**: Ensure the build output directory is `.svelte-kit/cloudflare` for SvelteKit on Pages (https://developers.cloudflare.com/pages/configuration/build-configuration/).

8. **Secrets Timing**: Secrets must be set before the deployment that uses them. Setting a secret doesn't trigger automatic redeployment (https://developers.cloudflare.com/pages/functions/bindings/).

### Verification Steps

1. **Check Build Output**: Verify `.svelte-kit/cloudflare` directory exists after `npm run build`

2. **Test Local Bindings**: Run `npm run preview` and check that D1/KV/R2 bindings are accessible in API routes

3. **Verify Migrations**: Check that tables exist:

   ```bash
   npx wrangler d1 execute eisen-db --remote --command="SELECT name FROM sqlite_master WHERE type='tables'"
   ```

4. **Check Deployment Logs**: View build logs in Cloudflare dashboard under Workers & Pages > your project > Deployments

5. **Test API Endpoints**: After deployment, test API routes to ensure bindings work:
   - `/api/sync` - Uses D1
   - `/api/backup` - Uses D1 and R2
   - `/api/pairing/initiate` - Uses D1 and KV

6. **Verify Environment Variables**: Check that `APP_NAME` and `RECORD_SCHEMA_VERSION` are accessible in your code

7. **Check PWA Functionality**: Verify service worker and PWA features work in production (the app uses `@vite-pwa/sveltekit`)

8. **Monitor Real User Metrics**: Use Cloudflare Analytics to monitor performance after deployment

### Troubleshooting Resources

- **Build Failures**: Check build logs for errors (https://developers.cloudflare.com/pages/configuration/debugging-pages/)
- **Binding Errors**: Verify binding names match between wrangler.toml and app.d.ts
- **Migration Errors**: Ensure you're using the correct `--local` or `--remote` flag
- **Runtime Errors**: Check Cloudflare dashboard > Workers & Pages > your project > Functions > Real-time logs

### Additional Notes

- The project uses Playwright for E2E testing (/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/playwright.config.ts). Run tests with `npm run test` after deployment.
- The app is a PWA using `@vite-pwa/sveltekit` (package.json:20). Ensure PWA assets are included in the build.
- For TypeScript support, the project references `./worker-configuration` in app.d.ts (line 2). Ensure this file exists or use `@cloudflare/workers-types` for full type support (https://svelte.dev/docs/kit/adapter-cloudflare).
