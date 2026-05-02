# Tutorial

This document describes the primary usage paths of `orion-error`, based on the current source code, tests, and `examples/`.

## Installation

```toml
[dependencies]
orion-error = "0.8.0"
```

Optional features:

```toml
[dependencies]
orion-error = { version = "0.8.0", features = ["serde"] }
orion-error = { version = "0.8.0", features = ["tracing"] }
orion-error = { version = "0.8.0", features = ["serde_json"] }
```

Default features: `derive`, `log`.

## Import Conventions

Prefer one of these two approaches:

**Application code (default):**
```rust
use orion_error::prelude::*;
use orion_error::reason::UnifiedReason;
use orion_error::runtime::OperationContext;
```

**Architecture boundaries** — explicit layered imports:
```rust
use orion_error::prelude::*;
use orion_error::conversion::*;    // cross-layer conversion
use orion_error::protocol::*;      // boundary output
use orion_error::protocol::*;      // boundary output
use orion_error::interop::*;       // std::error::Error bridge
```

## 1-Minute Example

```rust
use orion_error::prelude::*;
use orion_error::reason::UnifiedReason;
use orion_error::runtime::OperationContext;

#[derive(Debug, Clone, PartialEq, OrionError)]
enum AppReason {
    #[orion_error(identity = "biz.invalid")]
    Invalid,
    #[orion_error(transparent)]
    General(UnifiedReason),
}

fn load_config(path: &str) -> Result<String, StructError<AppReason>> {
    let ctx = OperationContext::doing("load_config")
        .with_field("path", path)
        .with_meta("component.name", "config_loader");

    std::fs::read_to_string(path)
        .source_err(AppReason::system_error(), "read failed")
        .doing("read file")
        .with_context(&ctx)
}
```

This covers the four core points:

- Domain reason defined with `OrionError`
- Error entry via `source_err(reason, detail)` (unified entry)
- Semantic context via `doing(...)`
- Diagnostic fields and metadata on `OperationContext`

## 1. Defining Reason

### 1.1 Domain Reason

New code should use `#[derive(OrionError)]`:

```rust
use orion_error::{OrionError, UnifiedReason};

#[derive(Debug, Clone, PartialEq, OrionError)]
enum OrderReason {
    #[orion_error(identity = "biz.order_not_found")]
    OrderNotFound,
    #[orion_error(identity = "biz.insufficient_funds")]
    InsufficientFunds,
    #[orion_error(transparent)]
    General(UnifiedReason),
}
```

`OrionError` generates: `Display`, `DomainReason`, `ErrorCode`, `ErrorIdentityProvider`.

### 1.2 Universal Reason

`UnifiedReason` is the built-in universal reason classification. Common constructors:

- `UnifiedReason::validation_error()`, `UnifiedReason::business_error()`
- `UnifiedReason::system_error()`, `UnifiedReason::network_error()`, `UnifiedReason::timeout_error()`
- `UnifiedReason::core_conf()`, `UnifiedReason::logic_error()`

### 1.3 Delegate Constructors

If your domain reason has a transparent `UnifiedReason` variant, all `UnifiedReason` constructors are generated automatically:

```rust
AppReason::system_error()          // instead of AppReason::from(UnifiedReason::system_error())
AppReason::validation_error()
```

## 2. Constructing StructError

### 2.1 Direct Construction

```rust
let err = StructError::from(UnifiedReason::validation_error())
    .with_detail("field `email` is required");
```

### 2.2 Builder

```rust
let err = StructError::builder(UnifiedReason::validation_error())
    .detail("field `email` is required")
    .context_ref(&ctx)
    .finish();
```

### 2.3 Attaching Source

```rust
let err = StructError::from(UnifiedReason::system_error())
    .with_detail("read config failed")
    .with_source(std::io::Error::other("disk offline"));
```

Preferred APIs: `with_source(...)`, `builder.source(...)`. These auto-route between `StdError` and `StructError` source types.

## 3. Using Context

`OperationContext` carries runtime context:

```rust
let ctx = OperationContext::doing("place_order")
    .with_field("order_id", "A-1001")
    .with_field("user_id", "42")
    .with_meta("component.name", "order_service");
```

Attach context to an error:

```rust
let result = check_inventory()
    .doing("check inventory")
    .with_context(&ctx);
```

Common field types:
- `with_field(...)` — human-readable diagnostic entries (appears in Display output)
- `with_meta(...)` — machine-oriented metadata (serialization only)

## 4. Error Entry and Cross-Layer Conversion

### 4.1 `source_err(reason, detail)` — Unified Entry

Works for both raw `std::error::Error` and already-structured `StructError` sources:

```rust
let err = std::fs::read_to_string("config.toml")
    .source_err(UnifiedReason::system_error(), "read config failed")
    .unwrap_err();
```

Supported source types: `std::io::Error`, `anyhow::Error` (with `anyhow` feature), `serde_json::Error` (with `serde_json` feature), `toml::de::Error` / `toml::ser::Error` (with `toml` feature), and custom `RawStdError` types via `raw_source(...)`.

### 4.2 `conv_err()` — Cross-Layer Reason Remap

When the upstream error is already structured and you only need to change the reason type:

```rust
fn upper_layer_call() -> Result<(), StructError<ServiceReason>> {
    lower_layer_call().conv_err()?;
    Ok(())
}
```

Requires `ServiceReason: From<RepoReason>`.

## 5. Error Objects Summary

| Object | Purpose | Entry Point |
|--------|---------|-------------|
| `StructError<R>` | Runtime carrier | Propagation |
| `DiagnosticReport` | Human diagnostics | `err.report()` |
| `ErrorProtocolSnapshot` | Protocol projection | `err.exposure(&policy)` |

Standard Error interop: `as_std()`, `into_std()`, `into_boxed_std()`, `into_dyn_std()`.

## 6. Stable Identity and Protocol Projection

### 6.1 Stable Identity

Each error variant has a permanent machine-readable name:

```rust
assert_eq!(ApiReason::InvalidInput.stable_code(), "biz.invalid_input");
assert_eq!(ApiReason::InvalidInput.error_category().as_str(), "biz");
```

Stable identity never changes — unlike display text, numeric codes, or Rust paths.

The identity prefix (`biz`, `sys`, `conf`, `logic`) also determines the default `ExposurePolicy` behaviour.

### 6.2 Protocol Projection

The same error produces different JSON shapes for different protocol boundaries:

```rust
let proto = err.exposure(&DefaultExposurePolicy);

// HTTP response — minimal, safe for external clients
proto.to_http_error_json();

// Log output — full context for debugging
proto.to_log_error_json();

// RPC response — hides internal detail
proto.to_rpc_error_json();

// CLI output — human-readable summary
proto.to_cli_error_json();
```

## 7. Testing

```rust
use orion_error::dev::testing::assert_err_identity;
use orion_error::reason::ErrorCategory;

let err = std::fs::read_to_string("config.toml")
    .source_err(UnifiedReason::system_error(), "read config failed")
    .unwrap_err();

assert_err_identity(&err, "sys.io_error", ErrorCategory::Sys);
```

Test helpers: `assert_err_code()`, `assert_err_category()`, `assert_err_identity()`, `assert_err_operation()`, `assert_err_path()`.

## 8. Best Practices

- Define domain reasons with `#[derive(OrionError)]`
- Use `source_err(reason, detail)` as the unified error entry point
- Use `conv_err()` for cross-layer reason conversion
- Use `identity_snapshot()` for stable identity inspection
- Use `exposure(...)` for protocol boundary output
- Use explicit interop APIs when entering `std::error::Error` ecosystem
