# orion-error

Structured error handling for Rust services with:

- layered universal error categories via `UvsReason`
- domain-specific error enums with stable `OrionError` identities
- contextual propagation via `OperationContext` and `ErrorWith`
- first-entry conversion via `IntoAs`
- structured cross-layer wrapping via `ErrorWrapAs`
- optional source-chain preservation for real underlying errors

[![CI](https://github.com/galaxio-labs/orion-error/workflows/CI/badge.svg)](https://github.com/galaxio-labs/orion-error/actions)
[![Coverage Status](https://codecov.io/gh/galaxio-labs/orion-error/branch/main/graph/badge.svg)](https://codecov.io/gh/galaxio-labs/orion-error)
[![crates.io](https://img.shields.io/crates/v/orion-error.svg)](https://crates.io/crates/orion-error)

## Installation

```toml
[dependencies]
orion-error = "0.7.0"
```

Optional features:

```toml
[dependencies]
orion-error = { version = "0.7.0", features = ["serde"] }
# or
orion-error = { version = "0.7.0", features = ["tracing"] }
# or
orion-error = { version = "0.7.0", features = ["serde_json"] }
# or
orion-error = { version = "0.7.0", features = ["anyhow"] }
# or
orion-error = { version = "0.7.0", features = ["toml"] }
```

Default features include `log` and `derive`.

`StructError<R>` no longer implements `std::error::Error`. Standard-error
ecosystem boundaries should use the explicit bridge APIs instead:

```rust
let owned_std = err.clone().into_std();
let borrowed_std = err.as_std();
let boxed_std = err.into_boxed_std();
```

Default builds should use `source_ref()`, `report()`, `snapshot()`, or the
bridge APIs instead of calling `std::error::Error::source(&err)` directly on
`StructError<R>`.

Current docs:

- [CHANGELOG.md](./CHANGELOG.md)
- [docs/tutorial.md](./docs/tutorial.md)
- [docs/reason-identity-guide.md](./docs/reason-identity-guide.md)
- [docs/protocol-contract.md](./docs/protocol-contract.md)
- [docs/stable-snapshot-schema.md](./docs/stable-snapshot-schema.md)
- [docs/thiserror-comparison.md](./docs/thiserror-comparison.md)
- [orion-error-derive/README.md](./orion-error-derive/README.md)

Release order on crates.io:

1. Publish `orion-error-derive` first.
2. Wait for crates.io index propagation.
3. Publish `orion-error`.

Import guidance:

- `orion_error::prelude::*` is the primary convenience wildcard import and intentionally exports only the main path: `OrionError`, `StructError`, `IntoAs`, `ErrorWith`, `ErrorWrapAs`, and `DefaultExposurePolicy`.
- Small root imports such as `orion_error::{StructError, OrionError}` are preferred when you want explicit imports for the main path only.
- `orion_error::advanced_prelude::*` is only for advanced protocol/schema checks and migration tests.
- Layered imports are available when code needs stricter responsibility boundaries:
  - `orion_error::runtime::*`
  - `orion_error::conversion::*`
  - `orion_error::snapshot::*`
  - `orion_error::report::*`
  - `orion_error::bridge::*`
  - `orion_error::reason::*`
  - `orion_error::testcase::*`
- `orion_error::compat_prelude::*` / `orion_error::compat_traits::*` are explicit legacy compatibility imports for `owe(...)`

For new code, prefer `orion_error::prelude::*` plus small layered imports for examples, and small root imports for production modules. Use layered imports when the module benefits from explicit runtime / snapshot / report / bridge / testcase boundaries.

Recommended import split:

- `reason::*` for `ErrorCode`, `ErrorCategory`, `ErrorIdentityProvider`, `UvsReason`
- `report::*` for `Visibility`, projection response types, and projection/rendering APIs
- `snapshot::*` for stable snapshot schema constants
- `bridge::*` for `raw_source` and `RawStdError`
- `testcase::*` for `assert_err_identity(...)` and other test helpers

Root exceptions that are still reasonable:

- `ErrorCode` and `ErrorIdentityProvider` remain valid root imports because those names are also derive-macro entry points.

## Recommended API

Current primary names:

- `DefaultExposurePolicy`
- `ExposurePolicy`
- `ExposureDecision`
- `ExposureView`
- `exposure_view()`
- `exposure_snapshot()`
- `to_exposure_snapshot_json()`

## Feature matrix

- `derive`
  Enables `#[derive(OrionError)]`.
- `log`
  Enables `OperationContext` log integration.
- `tracing`
  Switches `OperationContext` logging to `tracing`.
- `serde`
  Enables serde support for runtime, report, and snapshot structures.
- `serde_json`
  Enables JSON helper methods such as `to_stable_snapshot_json()` and `to_exposure_snapshot_json()`.
- `anyhow`
  Enables `anyhow::Error` integration for `into_as(...)`.
- `toml`
  Enables TOML error integration for `into_as(...)`.

## Quick Start

```rust
use derive_more::From;
use orion_error::{
    prelude::*,
    reason::UvsReason,
    runtime::OperationContext,
};

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum AppError {
    #[orion_error(identity = "biz.invalid_request")]
    InvalidRequest,
    #[orion_error(transparent)]
    Uvs(UvsReason),
}

fn load_config() -> Result<String, StructError<AppError>> {
    let mut ctx = OperationContext::doing("load_config");
    ctx.record_field("path", "config.toml");
    ctx.record_meta("config.kind", "app_config");
    ctx.record_meta("config.format", "toml");

    std::fs::read_to_string("config.toml")
        .into_as(AppError::from(UvsReason::system_error()), "read config file failed")
        .doing("read config file")
        .with_context(&ctx)
}
```

Notes:

- `DomainReason` is implemented by `OrionError`; reason enums should derive `OrionError` instead of relying on structural blanket impls.
- Derive `OrionError` on domain enums and declare stable `identity` with `#[orion_error(...)]`.
- Use `record_field(...)` / `record_meta(...)` on `OperationContext`; `with_context(...)` is the primary error-side API for full context frames.
- Default to `into_as(...)` for supported plain error sources entering the structured system the first time.
- Use `wrap_as(...)` when the upstream value is already `StructError<_>` and the upper layer wants a new reason boundary.
- Runtime propagation uses `StructError`; stable machine export uses `StableErrorSnapshot`; human diagnostics use `DiagnosticReport`.
- For export-layer work, prefer `snapshot().stable_export()` or, with the `serde_json` feature, `snapshot().to_stable_snapshot_json()`.
- For human-facing diagnostics and redaction, use `report()` / `render(...)` / `render_redacted(...)`.
- Use `into_std()` / `OwnedStdStructError::from(err)` / `as_std()` / `StdStructRef::from(&err)` when explicitly bridging a `StructError<_>` into the standard error ecosystem.
- Use `OwnedStdStructError::into_struct()` when you need to come back from the owned bridge to the structured runtime carrier.
- Use `into_dyn_std()` only when an owned, type-erased official bridge is required, such as an `anyhow::Error` boundary that must later be recognized by `into_as(...)`.
- Use `into_boxed_std()` when a boundary requires `Box<dyn std::error::Error + Send + Sync>`.
- Use `source_payload()` / `source_payload_kind()` only for read-only inspection of the source payload branch.
- Use legacy `owe(...)` only as a compatibility path for `Display`-only values.
- Prefer `with_std_source(...)` / `with_struct_source(...)` and `source_std(...)` / `source_struct(...)` in new code so the source branch stays explicit. `with_source(...)` and `builder.source(...)` remain available as compatibility helpers for automatic routing.

## Core Concepts

### 1. `UvsReason`

`UvsReason` is the built-in cross-project error taxonomy:

- Business layer: `ValidationError` `100`, `BusinessError` `101`, `NotFoundError` `102`, `PermissionError` `103`, `LogicError` `104`, `RunRuleError` `105`
- Infrastructure layer: `DataError` `200`, `SystemError` `201`, `NetworkError` `202`, `ResourceError` `203`, `TimeoutError` `204`
- Config/external layer: `ConfigError` `300`, `ExternalError` `301`

Useful helpers:

- `error_code()`
- `is_retryable()`
- `is_high_severity()`
- `category_name()`

### 2. `StructError<R>`

`StructError<R>` is the main structured wrapper around a domain reason `R`.

It carries:

- `reason`
- `detail`
- `position`
- context stack
- optional underlying `source`

Construction styles:

```rust
let err = StructError::from(UvsReason::validation_error())
    .with_detail("missing field: user_id");
```

```rust
let err = StructError::builder(UvsReason::validation_error())
    .detail("missing field: user_id")
    .position(location!())
    .finish();
```

With preserved source:

```rust
let err = StructError::builder(UvsReason::system_error())
    .detail("failed to read config")
    .source(std::io::Error::other("disk offline"))
    .finish();
```

For non-structured sources on an existing `StructError`, prefer:

```rust
let err = StructError::from(UvsReason::system_error())
    .with_detail("failed to read config")
    .with_std_source(std::io::Error::other("disk offline"));
```

### 3. Context Propagation

```rust
use orion_error::{
    conversion::ErrorWith,
    runtime::OperationContext,
};

let mut ctx = OperationContext::doing("process_order");
ctx.record_field("order_id", "123");
ctx.record_field("user_id", "42");

let result = do_work()
    .doing("validate order")
    .with_context(&ctx);
```

Rules of thumb:

- `OperationContext::doing("process_order")` is the primary naming path for the outermost goal.
- Chained `.doing("validate order")` on an error appends an inner path segment instead of replacing the outer goal.
- `doing(...)` writes the structured `action` field and keeps `target/path` as the compatibility projection; `want(...)` is a compatibility alias.
- Use `action_main()` / `locator_main()` to read the primary semantics; use `target_main()` / `target_path()` when you need the compatibility projection.
- Display and `serde` now expose both `Want` and `Path`, for example: `Want=process_order`, `Path=process_order / validate order`.

### 3.1 Typed Metadata

`OperationContext` can also carry machine-readable metadata for diagnostics and classification:

```rust
use orion_error::{OperationContext, StructError, UvsReason};
use orion_error::runtime::ErrorMetadata;

let ctx = OperationContext::doing("load sink defaults")
    .with_meta("config.kind", "sink_defaults")
    .with_meta("config.scope", "sink")
    .with_meta("parse.line", 1u32);

let err = StructError::from(UvsReason::config_error()).with_context(ctx);
assert_eq!(err.context_metadata().get_str("config.kind"), Some("sink_defaults"));
```

Recommended usage:

- Put stable classification hints such as `config.kind`, `config.scope`, `component.name`, `parse.line` into metadata.
- Keep metadata short and machine-readable.
- Keep long human-facing explanations in `detail`.
- Metadata is not rendered by default in `Display`.

### 4. Conversion Helpers

Default recommendation for plain `Result<T, E: Error>` entering the structured system:

```rust
use orion_error::{conversion::IntoAs, reason::UvsReason};

read_file().into_as(UvsReason::system_error(), "read file failed")?;
http_call().into_as(UvsReason::network_error(), "http call failed")?;
```

Use `raw_source(...)` only when you must explicitly mark a downstream opt-in raw `StdError` type as unstructured:

```rust
use std::fmt;

use orion_error::{
    bridge::{raw_source, RawStdError},
    conversion::IntoAs,
    reason::UvsReason,
};

#[derive(Debug)]
struct ThirdPartyError;

impl fmt::Display for ThirdPartyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "third-party failure")
    }
}

impl std::error::Error for ThirdPartyError {}
impl RawStdError for ThirdPartyError {}

let third_party_call = || -> Result<(), ThirdPartyError> { Err(ThirdPartyError) };

third_party_call()
    .map_err(raw_source)
    .into_as(UvsReason::system_error(), "third-party call failed")?;
```

`raw_source(...)` is intentionally conservative. It only accepts types that explicitly implement `RawStdError`; it is not a blanket `E: StdError` path, and it must not be used for `StructError<_>`.

This is the intended design:

- `IntoAs` stays behind a sealed `UnstructuredSource` entry
- built-in allowlisted raw errors implement `UnstructuredSource` directly
- unknown downstream raw `StdError` types may opt in explicitly through `RawStdError`
- `StructError<_>` cannot enter `raw_source(...)`, because downstream crates cannot implement `RawStdError` for external types

In other words, the explicit escape hatch is kept without reopening a blanket `E: StdError` path.

With the `anyhow` feature, `anyhow::Error` is still treated as an aggregated but unstructured error by default. The only structured exception is a top-level official `OwnedDynStdStructError` created from `StructError<_>::into_dyn_std()`. `orion-error` does not scan arbitrary `anyhow` source chains and does not guess third-party wrappers.

Use legacy `owe(...)` only when maintaining values that are not real error types and only implement `Display`. Import it from the explicit compat module:

```rust
use orion_error::{compat_prelude::ErrorOweBase, reason::UvsReason};

message_only_result.owe(UvsReason::validation_error())?;
other_message_only_result.owe(UvsReason::business_error())?;
```

For converting one `StructError<R1>` into another `StructError<R2>`, prefer `err_conv()`:

```rust
repo_call().err_conv()?;
```

`err_conv()` preserves context, detail, position, and source.

If the upper layer wants to redefine the reason instead of converting it, use `wrap_as(...)` to keep the lower `StructError` as `source`:

```rust
use orion_error::{conversion::ErrorWrapAs, reason::UvsReason};

repo_call().wrap_as(UvsReason::system_error(), "service call failed")?;
```

In other words:

- `into_as(...)` is for `Result<T, E>` where `E` is a real non-structured error type
- `err_conv()` is for `Result<T, StructError<R1>>` to `Result<T, StructError<R2>>`
- `wrap_as(...)` is for `Result<T, StructError<R1>>` when the upper layer wants a new reason boundary
- `err_wrap(...)` / `wrap(...)` are compatibility helpers; prefer `wrap_as(...)` in new code

If you want to attach a lower `StructError` directly and preserve its structured source frames, use `with_struct_source(...)`:

```rust
use orion_error::{
    conversion::ErrorWith,
    reason::UvsReason,
    runtime::{OperationContext, StructError},
};

let source = StructError::from(UvsReason::config_error()).with_context(
    OperationContext::doing("load sink defaults")
        .with_meta("config.kind", "sink_defaults")
);

let err = StructError::from(UvsReason::system_error())
    .with_context(
        OperationContext::doing("start engine").with_meta("component.name", "engine"),
    )
    .with_struct_source(source);

assert_eq!(err.context_metadata().get_str("component.name"), Some("engine"));
assert_eq!(
    err.source_frames()[0].metadata.get_str("config.kind"),
    Some("sink_defaults")
);
```

The same rule applies to the builder API: use `.source_struct(lower_err)` for `StructError<_>` sources, and `.source_std(err)` for ordinary non-structured errors.

## Reports and Redaction

Default `Display` should stay concise. Use this separation:

- Runtime propagation uses `StructError`.
- Stable machine export uses `StableErrorSnapshot`.
- Human diagnostics and redaction use `DiagnosticReport`.

Most application code can stay on `StructError` and call the high-level helpers:

```rust
use orion_error::{
    reason::UvsReason,
    report::{RedactPolicy, RenderMode},
    runtime::StructError,
};

struct SimplePolicy;

impl RedactPolicy for SimplePolicy {
    fn redact_key(&self, key: &str) -> bool {
        matches!(key, "password" | "config.secret")
    }

    fn redact_value(&self, _key: Option<&str>, _value: &str) -> Option<String> {
        Some("<redacted>".to_string())
    }
}

let err = StructError::from(UvsReason::config_error())
    .with_detail("load config failed")
    .with_context(
        orion_error::runtime::OperationContext::doing("load config")
            .with_meta("config.kind", "sink_defaults")
            .with_meta("config.secret", "/prod/secrets/api-key"),
    );

let report = err.report();
assert_eq!(report.root_metadata.get_str("config.kind"), Some("sink_defaults"));

let verbose = err.render(RenderMode::Verbose);
let redacted = err.render_redacted(RenderMode::Verbose, &SimplePolicy);

assert!(verbose.contains("config.secret"));
assert!(redacted.contains("<redacted>"));
```

Recommended usage:

- `snapshot().stable_export()` or, with the `serde_json` feature, `snapshot().to_stable_snapshot_json()` for stable machine export.
- `report()` for human diagnostic inspection.
- `render(RenderMode::Compact)` for short summaries.
- `render(RenderMode::Verbose)` for local diagnostics and debug output.
- `render_redacted(...)` before writing potentially sensitive diagnostics to logs or external systems.

## Logging

`OperationContext` supports optional logging integration.

```rust
use orion_error::op_context;
use orion_error::op_context;

let mut ctx = op_context!("sync-user").with_auto_log();
ctx.record_field("user_id", "42");
ctx.info("starting sync");

do_sync()?;
ctx.mark_suc();
```

Use `scoped_success()` if you want RAII-style success marking.

## Source Chain

If you use `with_std_source(...)`, `raw_source(...)`, or `into_as(...)`, the original error remains available:

```rust
use orion_error::{conversion::IntoAs, reason::UvsReason, runtime::StructError};

let err: StructError<UvsReason> = std::fs::read_to_string("config.toml")
    .into_as(UvsReason::system_error(), "read config failed")
    .unwrap_err();

assert!(err.source_ref().is_some());
assert!(std::error::Error::source(&err.as_std()).is_some());
assert!(err.root_cause().is_some());
```

You can also inspect the entire chain:

```rust
let chain = err.source_chain();
let frames = err.source_frames();
let pretty = err.display_chain();
```

With the `serde` feature, the default `Serialize for StructError` remains a compatibility runtime projection. It still includes:

- `want`
- `path`
- `source_frames`
- `source_message`
- `source_chain`

`source_frames` is the structured form of the chain. Each frame contains:

- `index`
- `message`
- optional `display`
- optional `type_name`
- optional `error_code`
- optional `reason`
- optional `want`
- optional `path`
- optional `detail`
- optional `metadata`
- `is_root_cause`

For `StructError` sources, `message` is the stable reason text and `display` carries the full formatted error. `debug` remains available on `SourceFrame` at runtime, but it is not serialized by default because `Debug` output may contain sensitive internal fields. `source_chain` is kept as a compatibility summary; new observability pipelines should prefer `source_frames`. `type_name` is best-effort and should not be treated as a complete or stable classification key.

The underlying trait object itself is still not serialized. For new export paths, prefer `err.snapshot()`, `err.report()`, or the stable snapshot JSON helpers.

If you use legacy `owe(...)` helpers, only the display string is copied into `detail`, so they are not the preferred path for normal Rust errors.

## `thiserror` Interop

`thiserror` is no longer required for the recommended path. Prefer `OrionError` for domain reasons because it generates display text, stable identity, category, and the legacy numeric code from one annotation.

Use `thiserror` only when an existing enum already depends on `std::error::Error` behavior or external APIs require a standard error type.

See [docs/thiserror-comparison.md](docs/thiserror-comparison.md).

## Migration Notes

Prefer these current names:

- `CwdGuard`-style example does not apply here; ignore older cross-project docs
- `OperationContext::record_field(...)` instead of deprecated `with(...)`
- `with_auto_log()` instead of deprecated `with_exit_log()`
- prefer `into_as(reason, detail)` for real `StdError` sources
- keep `owe(...)` only for legacy `Display`-only cases

## Validation

From crate root:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features -- --test-threads=1
cargo run --example order_case
cargo run --example logging_example --features log
```

## Chinese Notes

当前版本文档以源码为准，推荐优先参考：

- [docs/tutorial.md](docs/tutorial.md)
- [docs/LOGGING.md](docs/LOGGING.md)
- [docs/thiserror-comparison.md](docs/thiserror-comparison.md)

如果 README 与源码冲突，请以 `src/` 和测试为准。
