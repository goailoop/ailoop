#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"

if [[ -z "$VERSION" ]]; then
  VERSION="$(grep -m1 '^version = ' ailoop-core/Cargo.toml | sed -E 's/version = "([^"]+)"/\1/')"
fi

if [[ -z "$VERSION" ]]; then
  echo "error: could not determine version" >&2
  exit 1
fi

echo "Syncing versions to: $VERSION"

# Rust (workspace members)
for f in ailoop-core/Cargo.toml ailoop-cli/Cargo.toml; do
  sed -i -E "s/^version = \"[^\"]+\"/version = \"$VERSION\"/" "$f"
done

# Python (pyproject)
sed -i -E "s/^version = \"[^\"]+\"/version = \"$VERSION\"/" ailoop-py/pyproject.toml

# TypeScript (package.json + package-lock.json) via npm to keep lockfile consistent
(
  cd ailoop-js
  current="$(node -p "require('./package.json').version")"
  if [[ "$current" != "$VERSION" ]]; then
    npm version "$VERSION" --no-git-tag-version
  fi
  npm install --package-lock-only --ignore-scripts
)
