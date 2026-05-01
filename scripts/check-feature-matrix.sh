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
  "no-default-features (doc)" \
  cargo test --doc --no-default-features

run_case \
  "derive (doc)" \
  cargo test --doc --no-default-features --features derive

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
  "serde only (no derive)" \
  cargo test --lib --tests --no-default-features --features serde

run_case \
  "serde_json only (no derive)" \
  cargo test --lib --tests --no-default-features --features serde_json,serde

run_case \
  "anyhow" \
  cargo test --lib --tests --no-default-features --features anyhow,derive

run_case \
  "toml" \
  cargo test --lib --tests --no-default-features --features toml,derive

run_case \
  "all-features" \
  cargo test --all-features -- --test-threads=1
