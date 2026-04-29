# Repository Guidelines

## Project Structure & Module Organization
- `src/` hosts the library; `src/core/` defines shared error primitives (reason, context, domain, universal) and `src/traits/` holds conversion helpers.
- `src/lib.rs` re-exports the public surface; `src/testcase.rs` bundles test utilities for integration tests.
- `tests/` contains black-box scenarios such as `tests/test_error_owe.rs`; mirror real workflows when adding new cases.
- `examples/` offers runnable demos (`order_case`, `logging_example`); keep them compiling for docs.
- `docs/` stores tutorials and design notes; `.github/workflows/` runs fmt, clippy, tests, coverage, audit; `tasks/` is for roadmap items.

## Build, Test, and Development Commands
- `cargo build` — compile the library with default features.
- `cargo test --all-features -- --test-threads=1` — execute the full suite in the same mode as CI.
- `bash scripts/check-feature-matrix.sh` — verify the supported feature combinations (`default`, `serde`, `serde_json`, `tracing`, `all-features`).
- `bash scripts/check-doc-code.sh` — rustdoc-test the main docs (`README`, tutorial, reason guide) against the built crate artifacts.
- `bash scripts/check-v3-policy.sh` — scan for deprecated/compat drift and `Result<T, String>` regressions.
- `cargo fmt --all` and `cargo fmt --all -- --check` — format code or verify formatting.
- `cargo clippy --all-targets --all-features -- -D warnings` — lint with warnings treated as errors.
- `cargo run --example order_case` and `cargo run --example logging_example --features log` — validate examples; swap `log` for `tracing` as needed.

## Coding Style & Naming Conventions
- Target Rust 2021 with rustfmt defaults; no manual formatting exceptions.
- Keep modules/functions in `snake_case`; types, enums, traits in `PascalCase`.
- Favor safe patterns; avoid `panic!` in library code. Lean on `thiserror`, `derive_more`, `serde` for ergonomic derives.

## Testing Guidelines
- Place unit tests beside implementations; integration tests live in `tests/` and follow the `test_*.rs` naming.
- Assert semantic behavior: `ErrorCode`, `StructError`, `detail()`, `context()` expectations.
- Keep `src/lib.rs` compile-fail guards and export regression tests aligned with any public-surface change.
- When docs or examples change, rerun `bash scripts/check-doc-code.sh` so snippets stay source-accurate.
- Optional coverage: `cargo llvm-cov --all-features --workspace --html` for local reports.

## Commit & Pull Request Guidelines
- Use short imperative commit messages (examples: `fmt`, `cargo clippy`, `add tracing hook`).
- Pull requests must summarize motivation, list API changes, link issues, and confirm fmt/clippy/test runs; include doc or example updates when APIs shift.

## Architecture Overview
- Core type is `StructError<R>` layered with `UvsReason` for retry and severity metadata.
- Context is enriched via `WithContext` and `OperationContext`; transform flows live under `traits::*` (`ErrorOwe`, `ErrorConv`).
- Preserve small, explicit public exports through `lib.rs` to keep module boundaries clear.
