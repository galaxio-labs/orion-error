# orion-error

Structured error handling for Rust services with:

- layered universal error categories via `UvsReason`
- domain-specific error enums with stable `ErrorCode`
- contextual propagation via `OperationContext` and `ErrorWith`
- conversion helpers via `ErrorOwe`, `ErrorOweSource`, and `ErrorConv`
- cross-layer wrapping via `WrapStructError` and `ErrorWrap`
- optional source-chain preservation for real underlying errors

[![CI](https://github.com/galaxio-labs/orion-error/workflows/CI/badge.svg)](https://github.com/galaxio-labs/orion-error/actions)
[![Coverage Status](https://codecov.io/gh/galaxio-labs/orion-error/branch/main/graph/badge.svg)](https://codecov.io/gh/galaxio-labs/orion-error)
[![crates.io](https://img.shields.io/crates/v/orion-error.svg)](https://crates.io/crates/orion-error)

## Installation

```toml
[dependencies]
orion-error = "0.6"
```

Optional features:

```toml
[dependencies]
orion-error = { version = "0.6", features = ["serde"] }
# or
orion-error = { version = "0.6", features = ["tracing"] }
```

Default features include `log`.

## Quick Start

```rust
use derive_more::From;
use orion_error::{
    ContextRecord, ErrorCode, ErrorOweSource, ErrorWith, OperationContext, StructError, UvsReason,
};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, From)]
enum AppError {
    #[error("invalid request")]
    InvalidRequest,
    #[error("{0}")]
    Uvs(UvsReason),
}

impl ErrorCode for AppError {
    fn error_code(&self) -> i32 {
        match self {
            Self::InvalidRequest => 1000,
            Self::Uvs(reason) => reason.error_code(),
        }
    }
}

fn load_config() -> Result<String, StructError<AppError>> {
    let mut ctx = OperationContext::want("load_config");
    ctx.record("path", "config.toml");

    std::fs::read_to_string("config.toml")
        .owe_sys_source()
        .want("read config file")
        .with(&ctx)
}
```

Notes:

- `DomainReason` is usually implemented automatically when your enum satisfies `From<UvsReason> + Display + PartialEq`.
- Use `record(...)` on `OperationContext`; `with(...)` on the context itself is deprecated.
- Default to `owe_*_source()` for real error types; use legacy `owe_*()` only when the upstream error is merely `Display`.

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

### 3. Context Propagation

```rust
use orion_error::{ContextRecord, ErrorWith, OperationContext};

let mut ctx = OperationContext::want("process_order");
ctx.record("order_id", "123");
ctx.record("user_id", "42");

let result = do_work()
    .want("validate order")
    .with(&ctx);
```

Rules of thumb:

- `OperationContext::want("process_order")` defines the outermost goal for this call.
- Chained `.want("validate order")` on an error appends an inner path segment instead of replacing the outer goal.
- Display and `serde` now expose both `Want` and `Path`, for example: `Want=process_order`, `Path=process_order / validate order`.
- Use `target_main()` to read the outermost goal and `target_path()` to read the full path.

### 4. Conversion Helpers

Default recommendation for plain `Result<T, E: Error>`:

```rust
read_file().owe_sys_source()?;
http_call().owe_net_source()?;
```

Use legacy `owe_*()` only for sources that are not real error types and only implement `Display`:

```rust
parse_input().owe_validation()?;
message_only_result.owe_biz()?;
```

For converting one `StructError<R1>` into another `StructError<R2>`:

```rust
repo_call().err_conv()?;
```

`err_conv()` preserves context, detail, position, and source.

For wrapping a lower-layer `StructError` into a new upper-layer reason while keeping the old error as `source`:

```rust
repo_call().err_wrap(UvsReason::system_error())?;
```

## Logging

`OperationContext` supports optional logging integration.

```rust
use orion_error::{op_context, ContextRecord};

let mut ctx = op_context!("sync-user").with_auto_log();
ctx.record("user_id", "42");
ctx.info("starting sync");

do_sync()?;
ctx.mark_suc();
```

Use `scoped_success()` if you want RAII-style success marking.

## Source Chain

If you use `with_source(...)` or `owe_*_source()`, the original error remains available:

```rust
let err: StructError<UvsReason> = std::fs::read_to_string("config.toml")
    .owe_sys_source()
    .unwrap_err();

assert!(std::error::Error::source(&err).is_some());
assert!(err.root_cause().is_some());
```

You can also inspect the entire chain:

```rust
let chain = err.source_chain();
let frames = err.source_frames();
let pretty = err.display_chain();
```

With the `serde` feature, serialized output also includes:

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
- `is_root_cause`

For `StructError` sources, `message` is the stable reason text and `display` carries the full formatted error. `debug` remains available on `SourceFrame` at runtime, but it is not serialized by default because `Debug` output may contain sensitive internal fields. `source_chain` is kept as a compatibility summary; new observability pipelines should prefer `source_frames`. `type_name` is best-effort and should not be treated as a complete or stable classification key.

The underlying trait object itself is still not serialized.

If you use legacy `owe_*()` helpers, only the display string is copied into `detail`, so they are not the preferred path for normal Rust errors.

## `thiserror` Integration

Recommended pattern:

- use `thiserror` for domain enum definition
- include `Uvs(UvsReason)` as the bridge variant
- implement `ErrorCode`
- use `orion-error` for conversion, context, and classification

See [docs/thiserror-comparison.md](docs/thiserror-comparison.md).

## Migration Notes

Prefer these current names:

- `CwdGuard`-style example does not apply here; ignore older cross-project docs
- `OperationContext::record(...)` instead of deprecated `with(...)`
- `with_auto_log()` instead of deprecated `with_exit_log()`
- prefer `owe_*_source()` by default; keep `owe_*()` for `Display`-only cases

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
