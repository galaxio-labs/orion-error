#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

latest_match() {
  local pattern="$1"
  local match
  match="$(find target/debug/deps -name "$pattern" -print0 | xargs -0 ls -t 2>/dev/null | head -n 1 || true)"
  printf '%s' "$match"
}

ORION_LIB="$(latest_match 'liborion_error-*.rlib')"
DERIVE_MORE_LIB="$(
  {
    latest_match 'libderive_more-*.so'
    latest_match 'libderive_more-*.dylib'
    latest_match 'libderive_more-*.rlib'
  } | sed '/^$/d' | xargs ls -t 2>/dev/null | head -n 1 || true
)"

if [[ -z "${ORION_LIB:-}" || -z "${DERIVE_MORE_LIB:-}" ]]; then
  echo "[doc-code] missing built dependencies in target/debug/deps"
  echo "[doc-code] run cargo test --all-features --no-run first"
  exit 1
fi

run_doc_test() {
  local label="$1"
  local file="$2"

  echo "[doc-code] $label"
  rustdoc \
    --edition=2021 \
    --test "$file" \
    -L dependency=target/debug/deps \
    --extern orion_error="$ORION_LIB" \
    --extern derive_more="$DERIVE_MORE_LIB"
}

run_doc_test "README.md" README.md
run_doc_test "README.zh-CN.md" README.zh-CN.md
run_doc_test "docs/tutorial.md" docs/tutorial.md
run_doc_test "docs/reason-identity-guide.md" docs/reason-identity-guide.md
