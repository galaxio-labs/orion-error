# Report / Exposure Boundary

This document describes the responsibility boundary between `DiagnosticReport` and `ErrorProtocolSnapshot`.

## Current State

`category` and `code` have been removed from `DiagnosticReport`. Identity data now lives exclusively in `ErrorProtocolSnapshot.identity`. All exposure bridge methods on `DiagnosticReport` (`exposure_identity`, `http_status`, `visibility`, `default_hints`, `decision`, `exposure`, `to_exposure_json`) have been deleted.

`StructError<T>::report()` only requires `DomainReason`, not `ErrorIdentityProvider`.

## 1. Object Roles

| Object | Responsibility |
|--------|---------------|
| `StructError<R>` | Runtime propagation, source chain, context attachment |
| `DiagnosticReport` | Human diagnostic view, redaction, text rendering |
| `ErrorProtocolSnapshot` | Identity + exposure decision + report, user debug, protocol JSON projection |

## 2. Recommended Primary Paths

**Human diagnostics:**
```rust
let report = err.report();
let text = report.render();
```

**Protocol/projection:**
```rust
let proto = err.exposure(&policy);
let debug = proto.render_user_debug();
let http = proto.to_http_error_json()?;
```

## 3. Principles

- `DiagnosticReport` stays a diagnostic object.
- `ErrorProtocolSnapshot` is the sole exposure/projection closure.
- `StructError` routes runtime errors into either report or protocol layer.

In short:
- Need text diagnostics → `report()`
- Need exposure / JSON projection → `exposure(…)`

## 4. From DiagnosticReport to Protocol

If the caller starts from an existing `DiagnosticReport` (not `StructError`):

```rust
let proto = ErrorProtocolSnapshot::from_report_skeleton(report, identity, &policy);
```

But if full projection data (root metadata, source frames, path) is needed, prefer `StructError::exposure(...)`.

## 5. Summary

The current design keeps `DiagnosticReport` focused on diagnostics while `ErrorProtocolSnapshot` handles all exposure and projection concerns. The two paths are independent and should not be mixed.
