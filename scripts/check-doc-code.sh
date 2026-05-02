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

load_doc_deps() {
  ORION_LIB="$(latest_match 'liborion_error-*.rlib')"
  DERIVE_MORE_LIB="$(
    {
      latest_match 'libderive_more-*.so'
      latest_match 'libderive_more-*.dylib'
      latest_match 'libderive_more-*.rlib'
    } | sed '/^$/d' | xargs ls -t 2>/dev/null | head -n 1 || true
  )"
}

echo "[doc-code] preparing dependencies with cargo test --all-features --no-run"
cargo test --all-features --no-run
load_doc_deps

if [[ -z "${ORION_LIB:-}" || -z "${DERIVE_MORE_LIB:-}" ]]; then
  echo "[doc-code] failed to locate built dependencies in target/debug/deps"
  exit 1
fi

run_doc_test() {
  local label="$1"
  local file="$2"

  if [[ ! -f "$file" ]]; then
    echo "[doc-code] missing doc file: $file"
    exit 1
  fi

  echo "[doc-code] $label"
  rustdoc \
    --edition=2021 \
    --test "$file" \
    --cfg 'feature="serde"' \
    --cfg 'feature="serde_json"' \
    --cfg 'feature="derive"' \
    --cfg 'feature="log"' \
    -L dependency=target/debug/deps \
    --extern orion_error="$ORION_LIB" \
    --extern derive_more="$DERIVE_MORE_LIB"
}

run_doc_test "README.md" README.md
run_doc_test "README.zh-CN.md" README.zh-CN.md
run_doc_test "docs/zh/src/user/tutorial.md" docs/zh/src/user/tutorial.md
run_doc_test "docs/zh/src/user/reason-identity-guide.md" docs/zh/src/user/reason-identity-guide.md
run_doc_test "docs/en/src/user/tutorial.md" docs/en/src/user/tutorial.md
