# orion-error

[English](./README.md) | [简体中文](./README.zh-CN.md)

Structured error governance for large Rust codebases.

`orion-error` is not just an error type library.

It is a governance framework for large Rust services and multi-layer systems.
It helps teams move from ad-hoc strings and mixed local conventions to one
shared error model for:

- semantic modeling
- runtime propagation
- context attachment
- cross-layer conversion
- boundary-facing output for HTTP / RPC / CLI / logs

Core building blocks:

- stable business identities via `#[derive(OrionError)]`
- one runtime carrier: `StructError<R>`
- explicit first-entry conversion with `source_err(...)`
- unified error entry point: .source_err(...)` for all source types
- report, snapshot, and exposure helpers for service boundaries

[![CI](https://github.com/galaxio-labs/orion-error/workflows/CI/badge.svg)](https://github.com/galaxio-labs/orion-error/actions)
[![Coverage Status](https://codecov.io/gh/galaxio-labs/orion-error/branch/main/graph/badge.svg)](https://codecov.io/gh/galaxio-labs/orion-error)
[![crates.io](https://img.shields.io/crates/v/orion-error.svg)](https://crates.io/crates/orion-error)

## Why It Is Useful

Use this crate when you want:

- one shared error language across service / repo / adapter / protocol layers
- clear business error enums instead of scattered strings
- one consistent way to attach detail, source, and operation context
- stable machine-facing identity for HTTP / RPC / log / CLI boundaries
- controlled bridging to `std::error::Error` only where needed
- a system that scales better than local `Result<T, String>` habits

If you only need a tiny local enum inside one module, `thiserror` alone may be
enough. If your service has layers, external boundaries, and structured error
output, `orion-error` is the better fit.

In short:

- `thiserror` is a good local modeling tool
- `orion-error` is for project-wide error governance

## Install

```toml
[dependencies]
orion-error = "0.8"
```

Default features include `derive` and `log`.

Common optional features:

```toml
[dependencies]
orion-error = { version = "0.8", features = ["serde"] }
orion-error = { version = "0.8", features = ["serde_json"] }
orion-error = { version = "0.8", features = ["tracing"] }
orion-error = { version = "0.8", features = ["anyhow"] }
orion-error = { version = "0.8", features = ["toml"] }
```

## Quick Start

```rust
use derive_more::From;
use orion_error::{
    prelude::*,
    reason::UnifiedReason,
    runtime::OperationContext,
};

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum AppReason {
    #[orion_error(identity = "biz.invalid_request")]
    InvalidRequest,
    #[orion_error(transparent)]
    General(UnifiedReason),
}

fn load_config(path: &str) -> Result<String, StructError<AppReason>> {
    let ctx = OperationContext::doing("load_config")
        .with_field("path", path);

    std::fs::read_to_string(path)
        .source_err(AppReason::system_error(), "read config failed")
        .doing("read file")
        .with_context(&ctx)
}
```

What happens here:

- `AppReason` is your domain reason enum
- `StructError<AppReason>` is the runtime error carrier
- .source_err(...)` converts a normal Rust error into the structured system
- `doing(...)` and `with_context(...)` add operation context

For new code, treat `doing(...)` as the standard operation verb.

## The 4 APIs To Learn First

1. `#[derive(OrionError)]`
   Define stable business-facing reason enums.
2. .source_err(reason, detail)`
   Use when an error enters the structured system — works for both raw
   `std::error::Error` and already-structured `StructError<_>` sources.
3. `upcast()`
   Use when the upstream value is already `StructError<R1>` and you only remap
   reason type to `StructError<R2>`.
4. ~~`wrap_as(reason, detail)`~~ **Deprecated**: use `source_err` instead.

## Typical Flow

```text
raw std error ──→.source_err(...) ──→ first entry into structured system
                                          │
                                    upcast()
                                (reason remap)
                                          │
                  report / snapshot / exposure_snapshot
```

This is the important shift:

- lower layers do not invent random output shapes
- middle layers do not lose source and context
- boundary layers do not re-interpret raw strings
- the whole system shares one governance model

## Service Boundary Helpers

When you reach HTTP/RPC/log/CLI boundaries, these are the main entry points:

- `report()` for human-oriented diagnostics
- `snapshot().stable_export()` for stable machine export
- `exposure_snapshot(...)` with `to_http_error_json()`, `to_cli_error_json()`, `to_log_error_json()`, `to_rpc_error_json()`

Current protocol naming is `Exposure*`, not `ErrorPolicy*`.

That matters because large systems usually fail at the boundary:

- one team exposes too much detail
- another team hides everything
- every protocol builds its own error schema

`orion-error` gives those boundaries one consistent projection model.

## Third-Party Error Types

`source_err` supports built-in types (`io::Error`, `serde_json::Error`, `anyhow::Error`,
`toml::Error`) and custom types via opt-in:

```rust
use orion_error::interop::{raw_source, RawStdError};

#[derive(Debug)]
struct MyError;

impl std::fmt::Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "my custom error")
    }
}

impl std::error::Error for MyError {}

// Step 1: declare it as a raw source
impl RawStdError for MyError {}

// Step 2: wrap + convert
let result: Result<(), MyError> = Err(MyError);
let err = result
    .map_err(raw_source)
    .source_err(AppReason::system_error(), "my operation failed")
    .unwrap_err();

assert_eq!(err.source_ref().unwrap().to_string(), "my custom error");
```

> **Why opt-in instead of blanket `E: StdError`?** A blanket impl would silently
> swallow `StructError<_>` values as unstructured sources, losing their structured
> identity and context. The opt-in ensures you explicitly choose which types enter
> as unstructured sources versus structured ones.

**Newtype wrapper for foreign types.** If the error type comes from a dependency
and you cannot implement `RawStdError` directly (orphan rule), use a newtype:

```rust
use orion_error::interop::{raw_source, RawStdError};

struct WrappedError(reqwest::Error);

impl std::fmt::Display for WrappedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}
impl std::error::Error for WrappedError {}
impl RawStdError for WrappedError {}

// Usage
let result: Result<(), WrappedError> = Err(WrappedError(error));
let err = result
    .map_err(raw_source)
    .source_err(AppReason::system_error(), "api call failed")?;
```

## Standard Error Interop

`StructError<R>` no longer directly implements `std::error::Error`.

Use the explicit interop APIs when you need that ecosystem:

```rust
use orion_error::{StructError, UnifiedReason};

let borrowed_err = StructError::from(UnifiedReason::system_error());
let owned_err = StructError::from(UnifiedReason::system_error());
let boxed_err = StructError::from(UnifiedReason::system_error());

let borrowed_std = borrowed_err.as_std();
let owned_std = owned_err.into_std();
let boxed_std = boxed_err.into_boxed_std();

assert!(std::error::Error::source(&borrowed_std).is_none());
assert!(std::error::Error::source(&owned_std).is_none());
assert!(std::error::Error::source(boxed_std.as_ref()).is_none());
```

## Recommended Imports

For new code, start with:

```rust
use orion_error::prelude::*;
```

Treat this as the default for business code. Only switch to layered imports when
the module is explicitly modeling architecture boundaries, protocol adapters,
or test/schema checks.

Then add only the layered imports you need, for example:

- `orion_error::reason::UnifiedReason`
- `orion_error::runtime::OperationContext`
- `orion_error::runtime::source::*`
- `orion_error::report::*`
- `orion_error::protocol::*`
- `orion_error::snapshot::*`

This keeps normal application code on one predictable entry path while still
letting larger codebases keep clear module boundaries where that extra
precision is useful.

## Import Strategy

Three tiers:

**Application code (default)**
```rust
use orion_error::prelude::*;
use orion_error::reason::UnifiedReason;
use orion_error::runtime::OperationContext;
```

**Architecture boundaries** — use layered imports to make module coupling explicit.
```rust
// Domain layer
use orion_error::prelude::*;
use orion_error::reason::{ErrorCategory, ErrorIdentityProvider};

// Service / adapter layer — struct error is your carrier
use orion_error::{prelude::*, conversion::*};

// Protocol / boundary layer — output projection only
use orion_error::protocol::*;
use orion_error::report::{DiagnosticReport, RedactPolicy};
use orion_error::snapshot::*;

// Interop — when you must enter std::error::Error ecosystem
use orion_error::interop::*;
```

**Test / migration**
```rust
use orion_error::dev::prelude::*;
use orion_error::dev::testing::*;
```

## Error Flow Paths

There are exactly four ways a `StructError` enters or moves through your system:

```text
raw std error / StructError ──→.source_err(reason, detail) ──→ first entry
                                                                    │
                                                              upcast()
                                                          (reason remap)
                                                                    │
                                          report / snapshot / exposure_snapshot
```

**1. .source_err(reason, detail)`** — unified entry point. Works for both raw
   `std::error::Error` and already-structured `StructError` sources. Use this
   whenever an error enters your system.

**2. `upcast()`** — cross-layer conversion preserving semantics. The upstream error is
   already `StructError<R1>`; you only want to map the reason type to `StructError<R2>` via
   `From`. All detail, context, source, and metadata survive.

**4. `as_std() / into_std() / into_dyn_std()`** — exit point. Bridges the structured error
   into the `std::error::Error` ecosystem for interop or legacy interfaces. These are
   explicit; `StructError<T>` does not implement `StdError` directly.

## Try It

```bash
cargo test --all-features -- --test-threads=1
cargo run --example order_case
cargo run --example logging_example --features log
```

## Learn More

- [中文 README](./README.zh-CN.md)
- [Changelog](./CHANGELOG.md)
- [Docs Index](./docs/README.md)
- [Tutorial](./docs/user/tutorial.md)
- [Reason Identity Guide](./docs/user/reason-identity-guide.md)
- [Protocol Contract](./docs/user/protocol-contract.md)
- [Stable Snapshot Schema](./docs/user/stable-snapshot-schema.md)
- [thiserror Comparison](./docs/user/thiserror-comparison.md)
- [orion-error-derive README](./orion-error-derive/README.md)

## Maintainers

If publishing this crate family:

1. publish `orion-error-derive`
2. wait for crates.io index propagation
3. publish `orion-error`
