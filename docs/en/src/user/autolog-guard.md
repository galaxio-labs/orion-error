# Auto-Log Guard Usage

`orion-error` 0.9 splits operation logging into **data** and **guard** roles.
This page shows the recommended patterns; see [Logging](./LOGGING.md) for the
full reference.

## 1. Two roles, one job each

| Type | Role | Clone? | Drop side effect? |
| --- | --- | --- | --- |
| `OperationContext` | operation data (fields, path, result) | yes | **no** |
| `AutoLogGuard` | owns the data and writes the outcome log | **no** | yes (once) |

Because only the guard has a `Drop` side effect, cloning or attaching the
operation as data can never duplicate or defer the log. This is the fix for
[issue #64](https://github.com/galaxio-labs/orion-error/issues/64).

## 2. Quick start

```rust
use orion_error::OperationContext;

// 1. Build pure data.
let ctx = OperationContext::doing("sync_user").with_field("user_id", "42");

// 2. Arm a guard when you want lifecycle logging.
let guard = ctx.with_auto_log();

do_sync()?;          // if this returns early, nothing is marked...
guard.mark_success(); // ...so success is recorded only here.
```

On drop the guard writes `suc!`, `fail!`, or `cancel!`. The default result is
failure, so an un-marked guard logs `fail!`.

## 3. Recommended: `scope()` + `?`

`scope()` starts in the **failure** state, which makes it safe with `?`: an early
return skips `mark_success()`, so the failure is logged correctly.

```rust
use orion_error::OperationContext;

fn renew(gateway_id: &str) -> Result<(), MyError> {
    let mut guard = OperationContext::doing("renew credential")
        .with_auto_log()
        .with_field("gateway_id", gateway_id);

    let mut scope = guard.scope();   // default: failure
    let cred = fetch_credential(gateway_id)?; // early return -> fail! on drop
    store_credential(cred)?;                  // early return -> fail! on drop
    scope.mark_success();                     // only reached on success -> suc!
    Ok(())
}
```

Prefer this over `scoped_success()` for anything that uses `?`. `scoped_success()`
defaults to success, so a `?` early return would be logged as `suc!`:

```rust
// Only use when EVERY failure branch calls mark_failure().
let mut scope = guard.scoped_success();
if !validate() {
    scope.mark_failure();
}
```

## 4. Attaching operation data to an error safely

A very common move is to attach the operation to an error so the boundary can
render it. Because the data type is pure and the guard is not `Clone`, borrow the
guard — that copies data and emits **no** log:

```rust
use orion_error::OperationContext;

let guard = OperationContext::doing("renew")
    .with_auto_log()
    .with_field("gateway_id", gateway_id);

let value = fetch(gateway_id)
    .map_err(|e| e.with_context(&guard))?; // data copy; no log
guard.mark_success();
```

There is intentionally **no** by-value `with_context(guard)`: moving the guard
into the error would defer the log to the error's drop (or repeat it). If you
truly want to stop logging and keep only the data, disarm explicitly:

```rust
let data = guard.into_context(); // no log; plain OperationContext
```

## 5. Migrating from 0.8

`with_auto_log()` used to return `OperationContext`; it now returns
`AutoLogGuard`. Most call sites keep working unchanged because the guard derefs
to `OperationContext`:

```rust
// Before (0.8) and after (0.9) — same code, same behavior.
let mut ctx = OperationContext::doing("download").with_auto_log();
ctx.record("url", url);
ctx.mark_suc();          // works via DerefMut
```

What actually changes:

- Storing or returning the result of `with_auto_log()` **as `OperationContext`**
  no longer compiles. Use `into_context()` to recover the plain data, or keep the
  guard scoped to the operation.
- Code that relied on the *context* logging at its own drop (e.g. attaching an
  armed `OperationContext` to an error) should attach `&guard` instead.

## 6. Common pitfalls

- **Don't** store an `AutoLogGuard` in a long-lived struct just to log later;
  bind it to the operation scope it describes.
- **Don't** use `scoped_success()` in `?`-heavy code (see §3).
- **Do** attach with `&guard`, not by value.
- `#[must_use]` on the guard means an accidental bare `OperationContext::doing(..).with_auto_log();`
  produces a warning—if you did not want logging, drop the `with_auto_log()`.

## 7. Backend notes

The same API works with either backend:

- `log` (default): entries go through the `log` crate.
- `tracing`: with the `tracing` feature enabled, entries go through `tracing`
  (preferred when both features are on).

See [Logging](./LOGGING.md#1-feature) for feature configuration.
