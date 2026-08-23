#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

missing=0

for path in src package.json wrangler.toml CONTEXT.md docs/adr/013-web-one-password-e2ee.md; do
  if [[ ! -e "$path" ]]; then
    echo "ERROR: missing required path: $path" >&2
    missing=1
  fi
done

if [[ "$missing" -ne 0 ]]; then
  exit 1
fi

echo "Web app structure verified."
