#!/usr/bin/env bash
set -euo pipefail

# Run local checks equivalent to the CI pipeline.
# Requires: Android SDK configured for the Android checks.

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "=== Structure check ==="
./tools/verify-structure.sh

echo "=== Protocol stubs ==="
./tools/verify-protocol-stubs.sh

echo "=== Android lint ==="
(cd clients/android && ./gradlew lint)

echo "=== Android unit tests ==="
(cd clients/android && ./gradlew testDebugUnitTest)

echo "=== Local checks passed ==="
