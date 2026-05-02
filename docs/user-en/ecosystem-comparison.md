# Ecosystem Comparison: orion-error vs anyhow / thiserror / color-eyre

> Scope: anyhow / thiserror / color-eyre / orion-error

---

## 1. Positioning

| Dimension | anyhow | thiserror | color-eyre | **orion-error** |
|-----------|--------|-----------|------------|-----------------|
| **Positioning** | Quick error handling | Standard error derive | Diagnostic error reporting | **Structured error governance framework** |
| **Target users** | App developers (rapid prototyping) | Library authors | App developers (diagnostics) | **Large multi-team projects** |
| **Problem domain** | Reduce error handling boilerplate | Reduce Error impl boilerplate | Improve error diagnostic output | **Unified error modeling → runtime propagation → boundary protocol projection** |
| **Abstraction level** | Type erasure | Type-safe enum | Type erasure + diagnostics | **Generic structured carrier** |

---

## 2. Core Capabilities

### Error Definition

| Capability | anyhow | thiserror | color-eyre | orion-error |
|-----------|--------|-----------|------------|-------------|
| Custom error types | Not directly | `#[derive(Error)]` | Not directly | `#[derive(OrionError)]` |
| Generic error type | `Box<dyn Error>` | User-defined enum | `Box<dyn Error>` | `StructError<T: DomainReason>` |
| Stable identity | No | No | No | `stable_code()` + `ErrorCategory` |
| Numeric ErrorCode | No | Via `#[error(...)]` | No | Built-in `error_code()` |
| Display / source | Auto | Auto | Auto | Auto (`OrionError` derive) |

### Runtime Propagation

| Capability | anyhow | thiserror | color-eyre | orion-error |
|-----------|--------|-----------|------------|-------------|
| Context attachment | `.context()` / `.with_context()` | No | `.sections()` / `.note()` | `OperationContext` (doing/at/path + KV + metadata) |
| Context path | Single-layer context | No | Single-layer | **Multi-layer nested path** via `target_path()` |
| Custom metadata | No (message only) | No | `Section` trait | `ErrorMetadata` (typed KV, not in Display) |
| Source chain | Standard chain | Standard chain | Standard + `SpanTrace` | **Dual-channel** (Std/Struct) + rich `SourceFrame` |
| Cross-type conversion | `anyhow!()` macro | `#[from]` | `eyre!()` macro | `source_err()` / `conv_err()` |

### Boundary Output

| Capability | anyhow | thiserror | color-eyre | orion-error |
|-----------|--------|-----------|------------|-------------|
| Human diagnostics | `.display_chain()` | No | Colored output | `report().render()` + `RedactPolicy` |
| Protocol JSON (HTTP/RPC) | No | No | No | `exposure_snapshot()` → `to_*_error_json()` |
| Stable snapshot | No | No | No | `StableErrorSnapshot` + versioned schema |
| Exposure policy | No | No | No | `ExposurePolicy` (status/visibility/hints/retryable) |
| Redaction | No | No | Limited | `RedactPolicy` trait |

### std::error::Error Ecosystem

| Capability | anyhow | thiserror | color-eyre | orion-error |
|-----------|--------|-----------|------------|-------------|
| Implements StdError | Yes | Yes | Yes | **Explicit bridge** (`as_std()` / `into_std()`) |
| `dyn Error` compatible | Natively | Natively | Natively | Lossy (`OwnedDynStdStructError`) |
| Third-party interop | `.context()` / `anyhow!()` | `#[from]` | `.sections()` / `eyre!()` | `source_err()` / `raw_source()` |

---

## 3. Coexistence Strategy

| Layer | Recommended |
|-------|-------------|
| Outside boundary (3rd-party libs, FFI) | thiserror / standard Error trait |
| Entering structured system | orion-error `source_err()` |
| Business layer propagation | orion-error `StructError<R>` |
| Cross-layer (repo → service → handler) | orion-error `conv_err()` |
| Boundary output | orion-error `exposure_snapshot()` |
| Quick prototyping / glue code | anyhow (supported via `anyhow` feature) |
| Terminal diagnostics | orion-error `report().render()` or color-eyre |

## 4. When to Use What

### Choose orion-error

- Multi-layer Rust backend services (repo → service → handler → protocol)
- External HTTP/RPC/gRPC interfaces with unified error responses
- Microservices with stable error codes and monitoring classification
- Multi-team projects needing consistent error conventions
- Persistent/versioned error snapshots

### Choose alternatives

- Single-file scripts or CLI tools → anyhow
- Low-level libraries exposing `std::error::Error` → thiserror
- Terminal applications needing pretty error output → color-eyre
- Projects with only 1-2 layers, no structured context needed → thiserror + anyhow
