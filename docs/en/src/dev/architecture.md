# orion-error 0.8.0 Architecture

This document describes the ideal design architecture of orion-error `0.8.0`: the design constraints behind the public API, the core error flow, and the governance goals. Struct snippets are conceptual models, not exact source snapshots; the implementation in `src/` remains the source of truth for precise fields.

## The Problem

In large Rust services, error handling faces five unmet needs:

1. **Convergence without loss.** Lower-layer technical errors must be abstracted into upper-layer stable semantics — but the original cause (source chain, detail, context) must remain available for diagnostics.
2. **Cross-layer propagation.** An error passes through multiple layers (handler → service → repository → database). Each layer needs to attach its own context without discarding what came before.
3. **Boundary projection.** The same error must be presented differently to different audiences: end users (safe message), operators (component + retryability), protocol clients (stable code + structure), and developers (full chain).
4. **Governable identity.** Errors need stable, machine-readable identities that survive refactoring, across HTTP/RPC/log/CLI boundaries.
5. **Structured carrier.** Errors carry detail, source chain, operation context, and metadata — all as structured fields, not string concatenation.

Existing libraries solve a subset:

| Library | Strengths | Leaves open |
|---------|-----------|-------------|
| `thiserror` | Local error enum modeling, `Display` + `From` generation | Cross-layer propagation, context attachment, protocol projection |
| `anyhow` | Application-level error unification, `context()` | Stable identity, protocol output, fine-grained category routing |
| `color-eyre` | Rich diagnostic reports | Same as anyhow — no protocol or identity layer |

**orion-error** targets the gap: **governance at scale** — what happens when errors travel through 3–5 layers and must emerge at a protocol boundary with stable structure.

---

## Core Insight: Reason/Carrier Separation

The central design decision: **separate the error's semantic classification (reason) from its propagation mechanism (carrier).**

```rust
// Reason = what kind of error
enum AppReason {
    InvalidInput,
    OrderNotFound,
    General(UnifiedReason),
}

// Carrier = how it propagates
let err: StructError<AppReason> = AppReason::OrderNotFound
    .to_err()
    .with_detail("order #42 not found")
    .with_source(db_error)
    .with_context(ctx);
```

### Why separate?

If reason and carrier are combined — as in typical `thiserror` enum usage — every piece of runtime machinery (context attachment, source tracking, protocol projection) must be reimplemented for each enum. The carrier (`StructError<T>`) implements it once.

The reason stays thin — a `DomainReason` marker trait requiring only `PartialEq + Display + Debug + Send + Sync + 'static`. The carrier does the rest.

```rust
pub trait DomainReason: PartialEq + Display + Debug + Send + Sync + 'static {}
```

| Constraint | Reason |
|-----------|--------|
| `Display` + `Debug` | Errors must be printable for diagnostics and logging. |
| `PartialEq` | Enables assertion in tests. |
| `Send + Sync` | Required for `StructError` to cross async task boundaries. |
| `'static` | Enables type erasure via `dyn Error` and storage in `SourceFrame`. |

---

## Error Flow

```text
raw std error ──→ .source_err(reason, detail) ──→ first entry into structured system
                                                        │
                                                  conv_err()
                                              (reason remap)
                                                        │
                              report / exposure / display_chain
```

### 1. Entry: `source_err(reason, detail)`

The unified entry point. Works for both raw `std::error::Error` and already-structured `StructError` sources:

```rust
let result = std::fs::read_to_string("config.toml")
    .source_err(AppReason::system_error(), "read config failed")?;
```

- The raw error is stored as a source frame, preserving its `Display` and `Debug` output.
- The `reason` becomes the error's stable classification.
- The `detail` provides layer-specific explanation.

### 2. Cross-layer conversion: `conv_err()`

When the upstream error is already `StructError<R1>` and only the reason type needs to change:

```rust
fn upper_layer() -> Result<(), StructError<UpperReason>> {
    lower_layer().conv_err()?;
    Ok(())
}
```

Requires `UpperReason: From<LowerReason>`. All detail, context, source chain, and metadata survive the conversion.

A blanket `From<StructError<R1>> for StructError<R2>` is blocked by Rust's orphan rule (neither `From` nor `StructError` are local to the user's crate). An explicit trait method is the intended path.

### 3. First entry vs. cross-layer distinction

| Method | Semantics | Source preservation |
|--------|-----------|-------------------|
| `source_err(reason, detail)` | Creates a new semantic boundary | Wraps as unstructured or structured source |
| `conv_err()` | Only remaps reason type | Preserves all detail, context, source, metadata |

---

## Core Types

### `StructError<T: DomainReason>`

The universal runtime carrier. Conceptually, it stores the reason and the runtime propagation state behind a small carrier:

```rust
pub struct StructError<T: DomainReason> {
    imp: Box<StructErrorImpl<T>>,
}
```

`Box` is used to keep `StructError` small (pointer-sized), as it is expected to be returned through `Result` frequently.

### `StructErrorImpl<T>`

Holds the data needed for error propagation. Simplified model:

```rust
struct StructErrorImpl<T> {
    reason: T,
    detail: Option<String>,
    position: Option<String>,
    context: Option<Arc<Vec<OperationContext>>>,
    source_payload: Option<InternalSourcePayload>,
}
```

Key decisions:
- **`context: Option<Arc<Vec<...>>>`** — lazy allocation: no heap allocation for errors without context. `Arc` enables cheap clone of the context chain.
- **`Box<StructErrorImpl<T>>`** — `StructError` itself stays small (one pointer), minimizing `Result` size.

### `OperationContext`

Carries runtime context. Conceptually it describes what the current layer was doing, what it was accessing, which diagnostic fields were attached, and whether operation logging should be emitted:

```rust
pub struct OperationContext {
    action: Option<String>,
    locator: Option<String>,
    fields: Vec<(String, String)>,
    path: Vec<String>,
    metadata: ErrorMetadata,
    result: OperationResult,
    exit_log: bool,
}
```

- `doing(...)` — what operation was running ("load config", "validate order")
- `at(...)` — what resource was being accessed ("config.toml", "order #42")
- `with_field(...)` — human-readable diagnostic fields
- `with_meta(...)` — machine-oriented metadata (serialization only)
- `success()` / `fail()` / `cancel()` and logging helpers — record operation outcome with little call-site code

### `SourceFrame`

Represents one element in the source chain. Simplified model:

```rust
pub struct SourceFrame {
    pub index: usize,
    pub message: SmolStr,
    pub display: Option<SmolStr>,
    pub debug: Option<SmolStr>,
    pub type_name: Option<SmolStr>,
    pub error_code: Option<i32>,
    pub reason: Option<SmolStr>,
    pub path: Option<SmolStr>,
    pub detail: Option<SmolStr>,
    pub metadata: ErrorMetadata,
    pub is_root_cause: bool,
    pub context_fields: Vec<(SmolStr, SmolStr)>,
}
```

String fields use `SmolStr` (zero-allocation for short strings) for fast clone in source chain traversal.

---

## Consumption Paths

Three independent consumption paths, each returning a different view of the same error:

### `report()` → `DiagnosticReport`

Human-readable diagnostics. Only requires `DomainReason`.

```rust
let report: DiagnosticReport = err.report();
println!("{}", report.render());
```

Output:
```text
reason: system error
detail: read config failed
context:
  [0] place_order [user_id: 42]
```

### `exposure(&policy)` → `ErrorProtocolSnapshot`

Protocol-boundary projection. Requires `ErrorIdentityProvider` (provided by `#[derive(OrionError)]`).

```rust
let proto = err.exposure(&MyPolicy);
let http_json = proto.to_http_error_json()?;   // {"status": 500, "code": "sys.io_error", ...}
let log_json = proto.to_log_error_json()?;     // full structured log output
let cli_json = proto.to_cli_error_json()?;     // operator-facing summary
let rpc_json = proto.to_rpc_error_json()?;     // upstream-facing protocol
```

The `ExposurePolicy` trait controls the decision:

| Method | Default | Override frequency |
|--------|---------|-------------------|
| `http_status()` | 500 | Most common |
| `visibility()` | `Internal` (Biz → `Public`) | Common |
| `retryable()` | `false` | Occasional |
| `default_hints()` | `[]` | Rare |

`Visibility` controls which error information reaches the external caller:

| | `Public` | `Internal` |
|---|---------|-----------|
| HTTP `message` | Uses detail | Uses reason (hides detail) |
| RPC `detail` | Exposed | `null` |

### `display_chain()` → formatted string

Source chain expansion for debugging. No trait requirement beyond `DomainReason`.

```text
system error
  -> Info: read config failed
  -> Caused by:
      1. outer source
      2. inner source
```

### `identity_snapshot()` → `ErrorIdentity`

Stable identity inspection without protocol projection:

```rust
let id = err.identity_snapshot();
assert_eq!(id.code, "sys.io_error");
```

---

## UnifiedReason

`UnifiedReason` is the built-in universal reason classification. It covers the common error categories found in most services:

| Category | Code range | Examples |
|----------|-----------|---------|
| Business | 100-105 | `validation_error`, `not_found` |
| Infrastructure | 200-204 | `system_error`, `network_error`, `timeout` |
| Configuration | 300-301 | `core_conf`, `external_error` |

Designed as a catch-all for errors that don't need a domain-specific reason. Domain enums typically include it as a transparent variant:

```rust
#[derive(OrionError)]
enum AppReason {
    #[orion_error(identity = "biz.invalid")]
    Invalid,
    #[orion_error(transparent)]
    General(UnifiedReason),
}
```

The `#[orion_error(transparent)]` attribute delegates `stable_code()`, `error_category()`, and `Display` to the inner `UnifiedReason`.

---

## Explicit StdError Bridge

`StructError<T>` does **not** implement `std::error::Error`. This is intentional:

1. **Prevents accidental type erasure.** If `StructError` implemented `StdError`, calling code could unintentionally erase the reason type with `.into()` or `Box<dyn Error>`, losing structured identity.
2. **Keeps boundary crossing explicit.** When interop with `StdError` ecosystem is needed, the conversion is explicit:

```rust
let std_ref: StdStructRef<'_, AppReason> = err.as_std();
let owned: OwnedStdStructError<AppReason> = err.into_std();
let dyn_owned: OwnedDynStdStructError = err.into_dyn_std();
```

---

## Derive Macro

`#[derive(OrionError)]` generates the core trait implementations:

| Trait | Purpose | Source |
|-------|---------|--------|
| `Display` | Human-readable error message | From `message` attribute, or auto-generated from `identity` |
| `DomainReason` | Carrier compatibility | Empty marker impl |
| `ErrorCode` | Legacy numeric compatibility code | From `code` attribute, or default 500 |
| `ErrorIdentityProvider` | Stable code + category | From `identity` and `category` attributes |

### Attributes

| Attribute | Required? | Generates |
|-----------|-----------|-----------|
| `identity = "biz.foo"` | Yes (unless `transparent`) | `stable_code()` returns `"biz.foo"` |
| `category = Biz` | No (inferred from `identity` prefix) | `error_category()` returns specified category |
| `transparent` | Alternative to `identity` | Delegates all methods to inner type |
| `message = "..."` | No (auto-generated from `identity`) | Custom `Display` output |
| `code = ...` | No (default 500) | Legacy numeric `error_code()` |

Protocol outputs, log aggregation, and monitoring should use `ErrorIdentity.code` / `stable_code()` as the stable identity. `ErrorCode` is a numeric compatibility layer, not the recommended primary key for new external contracts.

### Transparent Variant Constructor Delegation

When an enum has a transparent variant wrapping `UnifiedReason`, all `UnifiedReason` constructors are generated as methods on the enum:

```rust
#[derive(OrionError)]
enum AppReason {
    #[orion_error(transparent)]
    General(UnifiedReason),
}

// Generated automatically:
AppReason::system_error()   // instead of AppReason::General(UnifiedReason::system_error())
AppReason::validation_error()
AppReason::not_found_error()
```

---

## Third-Party Error Integration

Third-party error types enter the structured system through `source_err()`. Supported types:

| Type | Feature | Mechanism |
|------|---------|-----------|
| `std::io::Error` | Built-in (no feature) | Direct `UnstructuredSource` impl |
| `serde_json::Error` | `serde_json` | Direct `UnstructuredSource` impl |
| `anyhow::Error` | `anyhow` | Attempts structured recovery, falls back to unstructured |
| `toml::de::Error` | `toml` | Direct `UnstructuredSource` impl |
| Custom types | — | Opt-in via `RawStdError` + `raw_source()` |

The opt-in design (`RawStdError`) prevents silent structured-to-unstructured downgrade:

```rust,ignore
impl RawStdError for MyError {}

let result: Result<(), MyError> = Err(MyError);
let err = result
    .map_err(raw_source)
    .source_err(AppReason::system_error(), "my operation failed")?;
```

---

## Design Evolution

### Naming: UvsReason → CommonReason → UnifiedReason

The built-in reason type went through three names:

- **`UvsReason`** — original name, meaning unclear to new users
- **`CommonReason`** — intermediate rename, but "common" sounded like "ordinary" rather than "unified"
- **`UnifiedReason`** — final name, reflecting its role: concrete errors converge (are unified) into this classification

The deprecated `pub type UvsReason = UnifiedReason;` alias is retained for migration compatibility.

### Variant name: Uvs → General

The transparent variant in domain enums was renamed to `General`:

```rust
// Before
Uvs(UnifiedReason),

// After
General(UnifiedReason),
```

`General` communicates "this is the catch-all for non-domain-specific errors" more clearly than the opaque `Uvs`.

### Consumption path convergence: snapshot is not the main path

The orion-error 0.8.0 architecture centers on `report()`, `exposure()`, `display_chain()`, and `identity_snapshot()`.

Stable machine identity is provided by `identity_snapshot()`. HTTP/RPC/CLI/log boundary output is handled by `exposure()` and `ErrorProtocolSnapshot`. Human diagnostics are handled by `report()`. This avoids making users learn a separate snapshot type hierarchy while preserving stable identity and protocol projection.

### API naming: exposure

Consistency with `report()`. The shorter name reflects the intent: expose this error at a boundary according to a policy, without making users first learn an internal snapshot model.

---

## Feature Gating

| Feature | Enables | Default |
|---------|---------|---------|
| `derive` | Proc-macro derive macros (`OrionError`, `ErrorCode`, `ErrorIdentityProvider`) | Yes |
| `log` | `OperationContext` log methods (`ctx.info()`, `.debug()`, `.warn()`, `.error()`) and `Drop` auto-logging | Yes |
| `tracing` | Tracing integration (preferred over `log` when both are enabled) | No |
| `serde` | `Serialize` / `Deserialize` on core types | No |
| `serde_json` | Protocol JSON projection methods (`to_http_error_json()`, etc.) | No |
| `anyhow` | `anyhow::Error` interop with structured source recovery | No |
| `toml` | `toml::de::Error` / `toml::ser::Error` interop | No |

---

## Project Structure

```
src/
  lib.rs              — Crate root, re-exports, layered modules
  core/
    domain.rs         — DomainReason trait
    reason.rs         — ErrorCode trait, ErrorCategory enum, ErrorIdentityProvider trait
    universal.rs      — UnifiedReason enum (built-in classification)
    error/
      carrier.rs      — StructError<T>, StructErrorImpl<T>
      builder.rs      — StructErrorBuilder<T>
      identity.rs     — ErrorIdentity struct, identity_snapshot()
      source_chain.rs — SourceFrame, source payload infrastructure
      std_bridge.rs   — StdStructRef, OwnedStdStructError, OwnedDynStdStructError
    context/
      types.rs        — OperationContext, OperationScope
      convert.rs      — ContextAdd trait
    metadata.rs       — ErrorMetadata, MetadataValue
    report/
      diagnostic.rs   — DiagnosticReport, redaction
      protocol.rs     — ErrorProtocolSnapshot, ExposurePolicy, Visibility
  traits/
    contextual.rs     — ErrorWith trait
    conversion.rs     — ConvErr, ConvStructError, ToStructError
    source_err.rs     — SourceErr, RawStdError, RawSource
  testing.rs          — Test assertion helpers
```

```
docs/
  en/book.toml        — English mdBook config
  en/src/             — English mdBook source
  zh/book.toml        — Chinese mdBook config
  zh/src/             — Chinese mdBook source
  index.html          — Language selector copied to site root
site/
  en/                 — Generated English book
  zh/                 — Generated Chinese book
```

---

## Constraints

### Orphan Rule

A blanket `From<StructError<R1>> for StructError<R2>` cannot be provided — neither `From` (std) nor `StructError` (this crate) are local to the user's crate. The explicit `conv_err()` method is the intended path:

```rust
let result: Result<(), StructError<UpperReason>> = lower_result.conv_err()?;
```

### Send + Sync

`DomainReason` requires `Send + Sync`. This is necessary for `StructError` to be used across async task boundaries and captured by `anyhow::Error` or `Box<dyn Error>`. For single-threaded use, this adds a small but unavoidable constraint.
