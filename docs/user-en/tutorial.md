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
    let mut ctx = OperationContext::doing("load_config");
    ctx.record_field("path", path);

    std::fs::read_to_string(path)
        .into_as(AppReason::from(UvsReason::system_error()), "read failed")
        .doing("read file")
        .with_context(&ctx)
}
```

## The 4 APIs To Learn First

1. `#[derive(OrionError)]` — Define stable business-facing reason enums.
2. `into_as(reason, detail)` — First entry point: a raw `std::error::Error` enters the structured system.
3. `upcast()` — Cross-layer conversion preserving semantics. The upstream error is already `StructError<R1>`; you only remap the reason type.
4. `wrap_as(reason, detail)` — Cross-layer wrapping with a new semantic boundary. The upstream error becomes the *source* of the new one.

## Error Flow Paths

```text
raw std error ──→ into_as(reason, detail)
                       │
              ┌───────┼───────┐
              ▼       ▼       ▼
         upcast()  wrap_as()  as_std / into_std / into_dyn_std
         (reason   (new       (std::error::Error bridge)
          remap)    boundary)
```

## Boundary Output

```rust
use orion_error::protocol::DefaultExposurePolicy;

let err: StructError<AppReason> = /* ... */;
let proto = err.exposure_snapshot(&DefaultExposurePolicy);

// HTTP response
proto.to_http_error_json();

// RPC response
proto.to_rpc_error_json();

// CLI output
proto.to_cli_error_json();

// Log output
proto.to_log_error_json();
```

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
