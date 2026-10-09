# Logging

`orion-error` logging is built around three roles:

- `OperationContext` — **pure data**: a `Clone` carrier of operation fields,
  metadata, path, and result. It has **no `Drop` side effect**.
- `AutoLogGuard` — an owned, **non-`Clone` guard** created by
  `OperationContext::with_auto_log()`. It writes the `suc!` / `fail!` /
  `cancel!` entry **exactly once**, on drop.
- `OperationScope` — a borrowed guard for marking the result (`mark_success()`),
  usable through `OperationContext` or an `AutoLogGuard`.

For example-driven guidance and migration notes, see
[Auto-Log Guard Usage](./autolog-guard.md).

## 1. Feature

```toml
[dependencies]
orion-error = { version = "0.9.0", features = ["log"] }
# or
orion-error = { version = "0.9.0", features = ["tracing"] }
```

Default features include `log`.

Behavior:
- `log` only: uses `log` macros
- `tracing` enabled: prefers `tracing`
- Both enabled: prefers `tracing`

## 2. Basic Usage

```rust
use orion_error::OperationContext;

let ctx = OperationContext::doing("order_processing")
    .with_field("order_id", "123")
    .with_field("amount", "100.0")
    .with_meta("component.name", "order_service");

ctx.info("start");
ctx.debug("payload prepared");
ctx.warn("slow upstream");
ctx.error("final failure");
ctx.trace("verbose trace");
```

Aliases: `log_info`, `log_debug`, `log_warn`, `log_error`, `log_trace`.

## 3. Automatic Result Logging

`with_auto_log()` consumes the data context and returns an `AutoLogGuard`:

```rust
use orion_error::OperationContext;

let ctx = OperationContext::doing("sync_user").with_field("user_id", "42");
let guard = ctx.with_auto_log();

do_sync()?;
guard.mark_success();
```

The guard writes its entry when it is dropped. The default result is `Fail`: if
neither `mark_success()` nor `cancel()` is called before drop, a `fail!` entry is
written.

### Data vs. guard

Because the guard is deliberately **not** `Clone`, the drop-time side effect can
never be duplicated. To attach the same operation to an error purely as data
(e.g. for `display_chain()`), borrow the guard — that copies only the data and
emits nothing:

```rust
let guard = OperationContext::doing("sync_user").with_auto_log();

let err = build_error().with_context(&guard); // data copy; no log
// ... the guard is still the single owner that logs on drop.
```

There is intentionally no by-value `with_context(guard)`: moving the guard into
an error would defer or repeat its log. Use `guard.into_context()` if you want
to disarm the guard and keep the plain data.

## 4. OperationScope

`OperationScope` marks the result of a borrowed context. It works on a bare
`OperationContext` or through an `AutoLogGuard` (via `Deref`).

```rust
use orion_error::OperationContext;

let mut guard = OperationContext::doing("sync_user").with_auto_log();

{
    let mut scope = guard.scope();
    scope.with_field("user_id", "42");
    validate()?;
    scope.mark_success();
}
```

Methods:
- `scope()` — default failure; must call `mark_success()` explicitly
- `scoped_success()` — default success; use `mark_failure()` or `cancel()` to override
- `mark_success()` — mark as success
- `mark_failure()` — revert to failure
- `cancel()` — mark as cancelled

An `AutoLogGuard` also exposes `mark_success()` / `mark_failure()` / `cancel()`
directly (without creating a nested scope).

## 5. Prefer `scope()` for Fallible Flows

`scope()` defaults to failure, so it is **safe with `?`**: if the function
returns early, no `mark_success()` runs and the failure is logged correctly.

```rust
let mut guard = OperationContext::doing("process_order").with_auto_log();

{
    let mut scope = guard.scope();   // default: failure
    let value = do_work().await?;    // early return -> fail! on drop
    scope.mark_success();            // success -> suc!
}
```

`scoped_success()` defaults to success on creation, so a `?` early return would
still be recorded as `suc!`. Only use it when every failure branch calls
`mark_failure()` explicitly:

```rust
{
    let mut scope = guard.scoped_success();
    let ok = validate_order();
    if !ok {
        scope.mark_failure();
    }
}
```

## 6. `op_context!` Macro

```rust
use orion_error::op_context;

let guard = op_context!("load_config").with_auto_log().with_field("path", "config.toml");
```

This macro expands `module_path!()` at the call site, adding more accurate module
paths to automatic result logs.

## 7. Best Practices

- Use `doing(...)` to name operations
- Use `with_field(...)` / `with_meta(...)` for chained construction
- Use `record_field(...)` / `record_meta(...)` only when a mutable reference already exists
- Use `with_auto_log()` only on scopes that need result logging
- For fallible logic with `?`, prefer `scope() + mark_success()`
- Attach context to errors via `&guard` (data copy) so logging stays owned by the guard
- Use `scoped_success()` only when failure paths are explicitly handled
