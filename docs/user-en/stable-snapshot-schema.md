# Stable Snapshot Schema

## 1. Three Objects

- `StructError<R>` — runtime carrier
- `ErrorSnapshot` — rich snapshot (runtime freeze)
- `StableErrorSnapshot` — stable machine export

Current schema version: `orion-error.snapshot.v3`

Constant: `orion_error::snapshot::STABLE_SNAPSHOT_SCHEMA_VERSION`

## 2. Entry Points

```rust
let snapshot = err.snapshot();
let stable = snapshot.stable_export();
```

Equivalent:

```rust
let stable = err.snapshot().into_stable_export();
let stable = StableErrorSnapshot::from(err.snapshot());
let stable = StableErrorSnapshot::from(&err);
```

## 3. ErrorSnapshot (Rich Snapshot)

Fields: `reason`, `detail`, `position`, `path`, `context`, `root_metadata`, `source_frames`

Contains more data than StableErrorSnapshot:

- `SnapshotContextFrame.fields`
- `SnapshotContextFrame.result`
- `SnapshotSourceFrame.display`
- `SnapshotSourceFrame.type_name`

Used for debugging, compatibility observation, intermediate conversion.

## 4. StableErrorSnapshot

Top-level fields: `schema_version`, `reason`, `detail`, `position`, `path`, `context`, `root_metadata`, `source_frames`

## 5. Stable Context Shape

`StableErrorSnapshot.context[]` fields:

- `target` — compat root target projection
- `action` / `locator` / `path` — stable runtime semantics
- `metadata`

Excluded from stable schema:
- `fields` — ad-hoc KV pairs
- `result` — operation result (always Fail at snapshot time)

## 6. Stable Source Frame Shape

`StableErrorSnapshot.source_frames[]` fields:

- `index`, `message`, `error_code`, `reason`, `path`, `detail`, `metadata`, `is_root_cause`

Excluded from stable schema:
- `display`, `type_name`, `debug`

## 7. Serialization

With `serde` feature: `StableErrorSnapshot` is serializable. `ErrorSnapshot::Serialize` outputs the stable schema shape directly.

With `serde_json` feature:

```rust
err.snapshot().to_stable_snapshot_json()
err.into_snapshot().to_stable_snapshot_json()
```

## 8. Stable Snapshot → Report

```rust
let report = stable.report();
```

Lossy projection:
- `context.fields` → empty
- `context.result` → `Fail` (default)
- `source_frames.display` → None
- `source_frames.type_name` → None

Suitable for: rendering, diagnostics, text observation.
Not suitable for: reconstructing full runtime carrier, restoring original source relationships.

## 9. Design Constraints

- Adding or removing stable fields requires evaluating whether `STABLE_SNAPSHOT_SCHEMA_VERSION` should be bumped.
- Changes to rich-snapshot-only fields do not trigger a schema version change.

## 10. Non-Goals

Stable snapshot does not support:
- Deserialization back to `StructError<R>`
- Typed round-trip
- Tagged union schema
- Full runtime source object reconstruction from JSON
