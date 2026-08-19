#!/bin/sh
set -e

if [ -n "$BETTER_AUTH_SECRET" ] || [ -n "$BETTER_AUTH_URL" ]; then
	cat > .dev.vars <<EOF
BETTER_AUTH_SECRET=${BETTER_AUTH_SECRET:-dev-secret-change-in-production-min-32-chars!!}
BETTER_AUTH_URL=${BETTER_AUTH_URL:-http://localhost:8788}
EOF
fi

npx wrangler d1 migrations apply eisen-web-db --local
exec npx wrangler pages dev .svelte-kit/cloudflare --ip 0.0.0.0 --port 8788
