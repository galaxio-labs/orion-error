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
- explicit first-entry conversion with `into_as(...)`
- explicit cross-layer wrapping with `wrap_as(...)`
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
    reason::UvsReason,
    runtime::OperationContext,
};

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum AppReason {
    #[orion_error(identity = "biz.invalid_request")]
    InvalidRequest,
    #[orion_error(transparent)]
    Uvs(UvsReason),
}

fn load_config(path: &str) -> Result<String, StructError<AppReason>> {
    let mut ctx = OperationContext::doing("load_config");
    ctx.record_field("path", path);

    std::fs::read_to_string(path)
        .into_as(AppReason::from(UvsReason::system_error()), "read config failed")
        .doing("read file")
        .with_context(&ctx)
}
```

What happens here:

- `AppReason` is your domain reason enum
- `StructError<AppReason>` is the runtime error carrier
- `into_as(...)` converts a normal Rust error into the structured system
- `doing(...)` and `with_context(...)` add operation context

For new code, treat `doing(...)` as the standard operation verb.

## The 4 APIs To Learn First

1. `#[derive(OrionError)]`
   Define stable business-facing reason enums.
2. `into_as(reason, detail)`
   Use when a plain error enters the structured system for the first time.
3. `upcast()`
   Use when the upstream value is already `StructError<R1>` and you only remap
   reason type to `StructError<R2>`.
4. `wrap_as(reason, detail)`
   Use when the upstream value is already `StructError<_>` and the upper layer
   wants a new semantic boundary.

## Typical Flow

```text
std::io::Error
  -> into_as(...)
StructError<RepoReason>
  -> upcast() or wrap_as(...)
StructError<ServiceReason>
  -> report() / snapshot().stable_export() / exposure_snapshot(...)
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

## Standard Error Interop

`StructError<R>` no longer directly implements `std::error::Error`.

Use the explicit interop APIs when you need that ecosystem:

```rust
use orion_error::{StructError, UvsReason};

let borrowed_err = StructError::from(UvsReason::system_error());
let owned_err = StructError::from(UvsReason::system_error());
let boxed_err = StructError::from(UvsReason::system_error());

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

- `orion_error::reason::UvsReason`
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
use orion_error::reason::UvsReason;
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
raw std error ──→ into_as(reason, detail) ──→ first entry into structured system
                                                   │
                              ┌────────────────────┼────────────────────┐
                              ▼                    ▼                    ▼
                     upcast()              wrap_as(reason,     as_std / into_std
                     (same semantics,        detail)             / into_dyn_std
                      only reason type       (new semantic       (boundary needs
                      remap)                 boundary, wraps     std::error::Error)
                                              existing as source)
```

**1. `into_as(reason, detail)`** — entry point. A raw `std::error::Error` enters the structured
   system for the first time. Use this once per boundary crossing (e.g. at a FFI boundary or
   when a library error enters your domain layer).

**2. `upcast()`** — cross-layer conversion preserving semantics. The upstream error is
   already `StructError<R1>`; you only want to map the reason type to `StructError<R2>` via
   `From`. All detail, context, source, and metadata survive.

**3. `wrap_as(reason, detail)`** — cross-layer wrapping with a new semantic boundary.
   The upstream error is already `StructError<R1>` and the upper layer needs its own reason.
   The original error becomes the *source* of the new one.

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
