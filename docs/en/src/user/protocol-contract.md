# Protocol Contract

## 1. Three-Layer Structure

1. **Stable identity**: `ErrorIdentity`
2. **Exposure decision**: `ExposureDecision`
3. **Output projections**: HTTP / CLI / log / RPC / user debug

Roles:

- `StructError<R>` — runtime propagation
- `ErrorIdentity` — stable identification
- `DiagnosticReport` — human diagnostics
- `ErrorProtocolSnapshot` — identity + decision + report assembly

## 2. Stable Identity

`ErrorIdentity`

Fields:

- `code` — stable machine key
- `category` — stable classification
- `reason` — stable human summary
- `detail` — variable description (not a key)
- `position` — source location
- `path` — stable path projection

Entry points:

- `StructError::identity_snapshot()`
- `assert_err_code(…)` — asserts **stable code string**, not numeric `error_code()`
- `assert_err_category(…)`
- `assert_err_identity(…)`

## 3. Exposure

`protocol::ExposureDecision`

Fields:

- `http_status`
- `visibility`
- `default_hints`
- `retryable`

Default policy (`DefaultExposurePolicy`):

| Category | http_status | visibility |
|----------|-------------|------------|
| Biz | 400 | Public |
| Conf / Logic / Sys | 500 | Internal |

`sys.network_error`, `sys.timeout` → `retryable = true`. All others `retryable = false`.

Entry points:

- `ExposurePolicy::decide(…)`
- `StructError::exposure(…)`
- `StructError::into_exposure(…)`

## 4. ErrorProtocolSnapshot

Fields:

- `identity`
- `decision`
- `report` (read-only via `report()`)

Entry points:

- `StructError::exposure(…)`
- `StructError::into_exposure(…)`

Use cases: test snapshot, gateway reprojection, unified protocol output, debug summary.

## 5. HTTP Projection

Requires `serde_json` feature.

JSON fields: `status`, `code`, `category`, `message`, `visibility`, `hints`

Rules:
- `Public` → `message` uses `detail`
- `Internal` → `message` uses stable `reason`

Entry: `ErrorProtocolSnapshot::to_http_error_json()`

## 6. CLI Projection

Requires `serde_json` feature.

JSON fields: `code`, `category`, `summary`, `detail`, `visibility`, `hints`

Rules:
- `summary` uses compact render
- `detail` uses verbose render

Entry: `ErrorProtocolSnapshot::to_cli_error_json()`

## 7. Log Projection

Requires `serde_json` feature.

JSON fields: `code`, `category`, `reason`, `detail`, `path`, `visibility`, `hints`, `root_metadata`, `context`, `source_frames`

Rules:
- Full `context` preserved
- Full `root_metadata` preserved
- Full `source_frames` preserved

Entry: `ErrorProtocolSnapshot::to_log_error_json()`

## 8. RPC Projection

Requires `serde_json` feature.

JSON fields: `status`, `code`, `category`, `reason`, `detail`, `visibility`, `hints`, `retryable`

Rules:
- `detail` only visible when `Public`
- `retryable` from exposure decision

Entry: `ErrorProtocolSnapshot::to_rpc_error_json()`

## 9. User Debug Summary

`render_user_debug(…)` is a human-readable debug summary, not a machine protocol.

Entry: `ErrorProtocolSnapshot::render_user_debug()`, `.render_user_debug_redacted(…)`

Use cases: local debugging, sample output, manual troubleshooting.

Not: HTTP message, stable JSON schema.

## 10. DiagnosticReport

`DiagnosticReport` does not require `ErrorIdentityProvider`. Suitable for text rendering, redaction, human diagnostics.

Entry: `StructError::report()`, `StructError::into_report()`

## 11. Recommended Consumption Path

1. Runtime propagation → `StructError<R>`
2. Stable identification → `identity_snapshot()`
3. Unified output → `exposure(…)`
4. Protocol output → projection API
5. Human summary → `render_user_debug(…)`

Avoid:
- Using `Display` text as protocol key
- Using CLI text as machine protocol
- Using raw `detail` as stable assertion
