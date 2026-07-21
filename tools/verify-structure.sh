#!/usr/bin/env bash
set -euo pipefail

# Verify P0.01 repository boundaries and ownership locations are present.

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

cd "$ROOT"

missing=0

for dir in clients/android clients/windows core protocol storage servers tests docs ops tools; do
  if [[ ! -d "$dir" ]]; then
    echo "ERROR: missing boundary directory: $dir" >&2
    missing=1
  elif [[ ! -f "$dir/README.md" && ! -f "$dir/.gitkeep" ]]; then
    echo "ERROR: boundary directory has no README.md or .gitkeep: $dir" >&2
    missing=1
  fi
done

if [[ ! -f "docs/REPOSITORY-BOUNDARIES.md" ]]; then
  echo "ERROR: missing docs/REPOSITORY-BOUNDARIES.md" >&2
  missing=1
fi

if [[ "$missing" -ne 0 ]]; then
  exit 1
fi

echo "Repository boundaries verified."
