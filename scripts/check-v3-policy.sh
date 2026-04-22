#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

failures=0
report_only=0

if [[ "${1:-}" == "--report-only" ]]; then
  report_only=1
fi

find_files() {
  find "$@" -type f \( -name '*.rs' -o -name '*.md' \)
}

scan_result_string() {
  perl -ne '
    BEGIN {
      our $last_file = "";
      our $line_no = 0;
    }
    if ($ARGV ne $last_file) {
      $last_file = $ARGV;
      $line_no = 0;
    }
    $line_no++;
    while (/Result<((?:[^<>]+|<[^<>]*>)+),\s*String\s*>/g) {
      print "$ARGV:$line_no:$&\n";
    }
  ' "$@"
}

run_check() {
  local description="$1"
  shift

  echo "[v3-policy] $description"
  if ! "$@"; then
    failures=1
  fi
}

check_no_output() {
  local description="$1"
  shift

  echo "[v3-policy] $description"
  local output
  local status=0
  output="$("$@" 2>&1)" || status=$?

  if [[ "$status" -gt 1 ]]; then
    echo "$output"
    failures=1
    return
  fi

  if [[ -n "$output" ]]; then
    echo "$output"
    failures=1
  fi
}

report_matches() {
  local description="$1"
  shift

  echo "[v3-policy] $description"
  local output
  local status=0
  output="$("$@" 2>&1)" || status=$?

  if [[ "$status" -gt 1 ]]; then
    echo "$output"
    failures=1
    return
  fi

  if [[ -n "$output" ]]; then
    echo "$output"
  else
    echo "[v3-policy] no matches"
  fi
}

run_check \
  "deny deprecated main-path usage in library/examples" \
  env RUSTFLAGS="-D deprecated" cargo check --all-features --lib --examples

check_no_output \
  "deny compat wildcard imports in src/ and examples/" \
  grep -R -n -E \
  --include='*.rs' \
  --exclude='lib.rs' \
  --exclude='owenance.rs' \
  'use .*compat_(prelude|traits)::\*' \
  src examples

check_no_output \
  "deny explicit compat module access in src/ and examples/" \
  grep -R -n -E \
  --include='*.rs' \
  --exclude='lib.rs' \
  --exclude='owenance.rs' \
  'compat_(prelude|traits)::' \
  src examples

check_no_output \
  "deny Result<T, String> in src/ and examples/" \
  scan_result_string $(find src examples -type f -name "*.rs")

report_matches \
  "report deprecated/compat usage in docs/" \
  sh -c '
    grep -R -n -E \
      --include="*.rs" \
      --include="*.md" \
      "compat_(prelude|traits)::|OperationContext::want\\(|OperationContext::with_want\\(|ErrorWith::want\\(|ErrorWith::with\\(|WithContext::want\\(|ctx\\.with\\(" \
      docs || true
  '

report_matches \
  "report Result<T, String> in docs/" \
  scan_result_string $(find_files docs)

report_matches \
  "report deprecated/compat usage in tests/" \
  sh -c '
    grep -R -n -E \
      --include="*.rs" \
      --include="*.md" \
      "compat_(prelude|traits)::|OperationContext::want\\(|OperationContext::with_want\\(|ErrorWith::want\\(|ErrorWith::with\\(|WithContext::want\\(|ctx\\.with\\(" \
      tests || true
  '

report_matches \
  "report Result<T, String> in tests/" \
  scan_result_string $(find_files tests)

if [[ "$failures" -ne 0 ]]; then
  echo "[v3-policy] failed"
  exit 1
fi

if [[ "$report_only" -eq 1 ]]; then
  echo "[v3-policy] report-only"
else
  echo "[v3-policy] ok"
fi
