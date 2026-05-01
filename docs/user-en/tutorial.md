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
use orion_error::reason::UvsReason;
use orion_error::runtime::OperationContext;
```

**Architecture boundaries** — explicit layered imports:
```rust
use orion_error::prelude::*;
use orion_error::conversion::*;    // cross-layer conversion
use orion_error::protocol::*;      // boundary output
use orion_error::snapshot::*;      // stable snapshot
use orion_error::interop::*;       // std::error::Error bridge
```

## 3-Minute Example

```rust
use orion_error::prelude::*;
use orion_error::reason::UvsReason;
use orion_error::runtime::OperationContext;

#[derive(Debug, Clone, PartialEq, OrionError)]
enum AppReason {
    #[orion_error(identity = "biz.invalid")]
    Invalid,
    #[orion_error(transparent)]
    Uvs(UvsReason),
}

fn load_config(path: &str) -> Result<String, StructError<AppReason>> {
    let ctx = OperationContext::doing("load_config")
        .with_field("path", path);

    std::fs::read_to_string(path)
        .into_as(AppReason::system_error(), "read failed")
        .doing("read file")
        .with_context(&ctx)
}
```

## The 4 APIs To Learn First

1. `#[derive(OrionError)]` — Define stable business-facing reason enums.
2. `into_as(reason, detail)` — Unified entry point: works for both raw `std::error::Error` and already-structured `StructError` sources.
3. `upcast()` — Cross-layer conversion preserving semantics. The upstream error is already `StructError<R1>`; you only remap the reason type.

## Error Flow Paths

```text
raw std error / StructError ──→ into_as(reason, detail)
                                      │
                                 upcast()
                             (reason remap)
                                      │
               report / snapshot / exposure_snapshot
```

## Stable Identity

Every error variant has a **stable machine-readable name** that never changes, even if the display text is updated:

```rust
#[derive(OrionError)]
enum ApiReason {
    #[orion_error(identity = "biz.invalid_input")]
    InvalidInput,
}

// This string is the contract — monitoring, clients, and gateways all rely on it:
assert_eq!(ApiReason::InvalidInput.stable_code(), "biz.invalid_input");
assert_eq!(ApiReason::InvalidInput.error_category().as_str(), "biz");
```

Compare unstable vs stable:

| Unstable | Stable |
|----------|--------|
| `"invalid input"` (display text may change) | `"biz.invalid_input"` (permanent) |
| `100` (numeric code collision) | `"biz.invalid_input"` (namespaced) |
| `ApiReason::InvalidInput` (Rust path may be refactored) | `"biz.invalid_input"` (independent of source code layout) |

The identity prefix (`biz`, `sys`, `conf`, `logic`) also determines the default `ExposurePolicy` behaviour.

## Protocol Projection

The same error produces **different JSON shapes** for different protocol boundaries — no manual mapping needed:

```rust
use orion_error::protocol::DefaultExposurePolicy;

let err = StructError::from(ApiReason::system_error())
    .with_detail("disk offline at /dev/sda");

let proto = err.exposure_snapshot(&DefaultExposurePolicy);

// HTTP response — minimal, safe for external clients
let http = proto.to_http_error_json().unwrap();
assert_eq!(http["status"], 500);           // internal error
assert_eq!(http["message"], "system error"); // uses reason, NOT detail

// Log output — full context for debugging
let log = proto.to_log_error_json().unwrap();
assert_eq!(log["detail"], "disk offline at /dev/sda");   // full detail
assert!(log["source_frames"].is_array());                  // source chain

// RPC response — hides internal detail
let rpc = proto.to_rpc_error_json().unwrap();
assert_eq!(rpc["detail"], serde_json::Value::Null); // internal → detail hidden

// CLI output — human-readable summary
let cli = proto.to_cli_error_json().unwrap();
assert_eq!(cli["summary"], "system error: disk offline at /dev/sda");
```

**The key insight**: the error is a 3D object; each protocol boundary sees a different 2D shadow. The `ExposurePolicy` decides which surface is visible to whom.

## Stable Snapshot

```rust
let snapshot = err.snapshot();          // structured snapshot
let stable: StableErrorSnapshot = snapshot.stable_export();  // versioned export
```

## Standard Error Interop

`StructError<T>` does **not** implement `std::error::Error` directly. Use explicit bridge APIs:

```rust
let std_err = err.as_std();            // borrow
let owned_std = err.into_std();         // owned
let dyn_std = err.into_dyn_std();       // type-erased
```

## Key Principles

- Lower layers do not invent their own output shapes.
- Middle layers do not lose source and context.
- Boundary layers do not re-interpret raw strings.
- The whole system shares one governance model.
