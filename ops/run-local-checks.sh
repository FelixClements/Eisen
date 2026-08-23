#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "=== Structure check ==="
./tools/verify-structure.sh

echo "=== Web check ==="
npm ci
npm run check

echo "=== Web tests ==="
npm run test

echo "=== Web build ==="
npm run build

echo "=== Local checks passed ==="
