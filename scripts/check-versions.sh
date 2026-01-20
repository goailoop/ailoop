#!/usr/bin/env bash
set -euo pipefail

core="$(grep -m1 '^version = ' ailoop-core/Cargo.toml | sed -E 's/version = "([^"]+)"/\1/')"
cli="$(grep -m1 '^version = ' ailoop-cli/Cargo.toml | sed -E 's/version = "([^"]+)"/\1/')"
py="$(grep -m1 '^version = ' ailoop-py/pyproject.toml | sed -E 's/version = "([^"]+)"/\1/')"
js="$(node -p "require('./ailoop-js/package.json').version")"
lock="$(node -p "require('./ailoop-js/package-lock.json').version")"

fail=0

check_eq() {
  local name="$1"
  local val="$2"
  if [[ "$val" != "$core" ]]; then
    echo "version mismatch: $name=$val core=$core" >&2
    fail=1
  fi
}

check_eq "ailoop-cli" "$cli"
check_eq "ailoop-py" "$py"
check_eq "ailoop-js" "$js"
check_eq "ailoop-js:package-lock" "$lock"

if [[ "$fail" -ne 0 ]]; then
  exit 1
fi

echo "ok: all versions match $core"
