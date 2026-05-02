# Logging

`orion-error` logging capabilities are built around `OperationContext` and `OperationScope`.

## 1. Feature

```toml
[dependencies]
orion-error = { version = "0.8.0", features = ["log"] }
# or
orion-error = { version = "0.8.0", features = ["tracing"] }
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

```rust
use orion_error::OperationContext;

let mut ctx = OperationContext::doing("sync_user")
    .with_auto_log()
    .with_field("user_id", "42");

do_sync()?;
ctx.mark_suc();
```

Default result is `Fail`. If `with_auto_log()` is enabled but neither `mark_suc()` nor `mark_cancel()` is called before drop, a failure log is emitted.

## 4. OperationScope

`OperationScope` is a guard for scoped lifecycle management.

```rust
use orion_error::OperationContext;

let mut ctx = OperationContext::doing("sync_user").with_auto_log();

{
    let mut scope = ctx.scope();
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

## 5. When to Use `scoped_success()`

`scoped_success()` is suitable when:

- The scope already handles failure branches internally
- Failure is explicitly handled via `mark_failure()`
- The code does not use `?` to return early

Example:

```rust
let mut ctx = OperationContext::doing("process_order").with_auto_log();

{
    let mut scope = ctx.scoped_success();
    let ok = validate_order();
    if !ok {
        scope.mark_failure();
    }
}
```

Not recommended:

```rust,ignore
let mut scope = ctx.scoped_success();
validate()?;
```

Because `scoped_success()` defaults to success on creation. If `?` returns early, the scope is still marked as success on drop.

For fallible flows with early returns, prefer:

```rust
let mut scope = ctx.scope();
validate()?;
scope.mark_success();
```

## 6. `op_context!` Macro

```rust
use orion_error::op_context;

let ctx = op_context!("load_config").with_auto_log().with_field("path", "config.toml");
```

This macro expands `module_path!()` at the call site, adding more accurate module paths to automatic result logs.

## 7. Best Practices

- Use `doing(...)` to name operations
- Use `with_field(...)` / `with_meta(...)` for chained construction
- Use `record_field(...)` / `record_meta(...)` only when a mutable reference already exists
- Use `with_auto_log()` only on scopes that need result logging
- For fallible logic with `?`, prefer `scope() + mark_success()`
- Use `scoped_success()` only when failure paths are explicitly handled
