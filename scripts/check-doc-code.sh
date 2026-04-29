#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

ORION_LIB="$(find target/debug/deps -name 'liborion_error-*.rlib' | head -n 1)"
DERIVE_MORE_LIB="$(find target/debug/deps \( -name 'libderive_more-*.so' -o -name 'libderive_more-*.dylib' -o -name 'libderive_more-*.rlib' \) | head -n 1)"

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
