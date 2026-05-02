# Release Checklist

Steps for publishing `0.8.x`.

## Pre-release

1. Confirm `CHANGELOG.md`, `README.md`, `docs/` are in sync with current code.
2. Confirm `orion-error` and `orion-error-derive` have matching versions.
3. Run:
   - `cargo fmt --all`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --all-features -- --test-threads=1`
   - `cargo test --doc --no-default-features`
   - `bash scripts/check-feature-matrix.sh`
   - `bash scripts/check-doc-code.sh`
   - `bash scripts/check-v3-policy.sh`
4. In a networked environment:
   - `cargo package --manifest-path orion-error-derive/Cargo.toml`
   - `cargo package`
   - `cargo publish --manifest-path orion-error-derive/Cargo.toml --dry-run`
   - `cargo publish --dry-run`

## Pre-release Boundary Checks

1. `src/lib.rs` root surface compile-fail doctests still pass.
2. `tests/test_layered_exports.rs`, `tests/test_versioned_namespaces.rs` still cover current layered export boundaries.
3. README / tutorial / reason identity guide code blocks match current source.
4. New or migrated public surface: add tests / compile guards first, then update README / docs, then update changelog.

## Publishing Order

1. Publish `orion-error-derive` first.
2. Wait for crates.io index propagation.
3. Publish `orion-error`.

The GitHub Actions release workflow is already configured in this order.

## Post-release

1. Confirm both crates are visible on crates.io.
2. Confirm the default `derive` feature correctly resolves `orion-error-derive`.
3. Confirm docs.rs pages generate:
   - `orion-error`
   - `orion-error-derive`
