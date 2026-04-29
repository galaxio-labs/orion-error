#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

run_case() {
  local label="$1"
  shift

  echo "[feature-matrix] $label"
  "$@"
}

run_case \
  "default" \
  cargo test --lib --tests

run_case \
  "serde" \
  cargo test --lib --tests --no-default-features --features serde,derive

run_case \
  "serde_json" \
  cargo test --lib --tests --no-default-features --features serde,serde_json,derive

run_case \
  "tracing" \
  cargo test --lib --tests --no-default-features --features tracing,derive

run_case \
  "all-features" \
  cargo test --all-features -- --test-threads=1
