# Why orion-error

`orion-error` is not about prettier error printing. It is about making errors in Rust services governable, traceable, exposable, and evolvable as structured contracts.

Basic error handling answers “how does this function return failure?” Larger services need stronger answers:

- How does an error carry the environment in which it happened?
- How do lower-level technical failures become stable upper-layer semantics without losing diagnostics?
- How can debugging see the error chain across layers?
- How can logs stay useful without logging the same failure everywhere?
- How should the same error be shown differently to users, operators, developers, and protocol clients?

`orion-error` keeps errors structured as they cross layers instead of reducing them to strings.

---

## 1. Diagnostics

### 1.1 Low-level errors often miss critical environment

Low-level errors usually describe the technical failure, not the business environment.

```rust
let content = std::fs::read_to_string(path)?;
```

If this fails, `std::io::Error` may say:

```text
No such file or directory
```

But debugging often needs to know:

- which path was read
- which operation was running
- which tenant, order, request, or component was involved
- whether the file was config, an order record, cache, or temporary data
- whether the failure should be classified as config, system, or validation failure

### Weak approach: add context only to logs

```rust
match std::fs::read_to_string(path) {
    Ok(content) => Ok(content),
    Err(err) => {
        log::error!("read config failed, path={path}, error={err}");
        Err(err)
    }
}
```

This splits diagnostics between logs and the error value. The caller still receives an error without structured context.

### Recommended approach: attach structured context to the error

```rust
use orion_error::prelude::*;
use orion_error::runtime::OperationContext;

let ctx = OperationContext::doing("load config")
    .with_field("path", path.display().to_string())
    .with_meta("component.name", "config_loader");

let content = std::fs::read_to_string(path)
    .source_err(AppReason::system_error(), "read config failed")
    .with_context(&ctx)?;
```

Here:

- `source_err(...)` brings `std::io::Error` into the structured error system.
- `AppReason::system_error()` is the stable upper-layer reason.
- `"read config failed"` is this layer’s explanation.
- `OperationContext` carries fields such as `path` and `component.name`.

### 1.2 Technical failures must be abstracted without losing diagnostics

Two common approaches are both poor:

1. Drop the lower-level error.
2. Expose the lower-level error directly and let higher layers depend on the repository’s technical choice.

The better approach is: **at layer boundaries, convert the lower-level failure into the current layer’s stable error semantics while preserving source, detail, and context for diagnostics.**

#### Weak approach: leak implementation errors upward

```rust
async fn submit_order(order: Order) -> Result<(), sqlx::Error> {
    repository::insert_order(order).await?;
    Ok(())
}
```

Now the service/API layer knows the repository uses `sqlx`.

#### Weak approach: remove the root cause

```rust
async fn submit_order(order: Order) -> Result<(), StoreError> {
    if repository::insert_order(order).await.is_err() {
        return Err(StoreReason::Unavailable.to_err());
    }

    Ok(())
}
```

This hides implementation details, but also removes the original cause.

#### Recommended approach: abstract the reason and preserve source

```rust
use orion_error::prelude::*;

async fn write_order(order: Order) -> Result<(), StructError<StoreReason>> {
    repository::insert_order(&order)
        .await
        .source_err(StoreReason::Unavailable, "insert order failed")
        .with_field("order_id", order.id.to_string())
        .with_meta("component.name", "order_store")?;

    Ok(())
}
```

The caller sees `StructError<StoreReason>`, not `sqlx::Error`, while the original database error remains available as internal source.

If the upper layer only remaps reason type, use `conv_err()`:

```rust
async fn submit_order(order: Order) -> Result<(), StructError<AppReason>> {
    write_order(order).await.conv_err()?;
    Ok(())
}
```

If the upper layer creates a new semantic boundary, use `source_err(...)`:

```rust
async fn submit_order(order: Order) -> Result<(), StructError<AppReason>> {
    write_order(order)
        .await
        .source_err(AppReason::system_error(), "submit order failed")?;

    Ok(())
}
```

### 1.3 Debugging needs an error chain, not an isolated message

Real failures often travel through multiple layers:

```text
HTTP handler
  -> service
    -> repository
      -> database / filesystem / remote API
```

A final message such as:

```text
submit order failed
```

is not enough. Debugging needs the path:

```text
submit order failed
  caused by: insert order failed
  caused by: database request failed
  caused by: connection timed out
```

The chain answers:

- what the original technical failure was
- which layers interpreted it
- where new semantic boundaries were introduced
- what context each layer added
- how the external error relates to the internal root cause

#### Recommended approach: preserve source chain at boundaries

```rust
async fn adapter_call(req: Request) -> Result<Response, StructError<AdapterReason>> {
    client.send(req)
        .await
        .source_err(AdapterReason::RemoteUnavailable, "remote call failed")
}

async fn load_quote(id: QuoteId) -> Result<Quote, StructError<ServiceReason>> {
    adapter_call(Request::quote(id))
        .await
        .source_err(ServiceReason::QuoteLoadFailed, "load quote failed")
        .with_field("quote_id", id.to_string())?;

    todo!("map response")
}
```

This preserves service semantics, adapter semantics, the lower source, and structured fields.

---

## 2. Operations

### Good logging is boundary logging, not more logging

Logging often becomes noisy because each layer emits its own `error!` line:

```rust
log::error!("repository insert failed: {err}");
log::error!("service submit failed: {err}");
log::error!("http request failed: {err}");
```

This duplicates failures and forces operators to reconstruct the chain manually.

The better model is: **the error carries identity, reason, detail, context, and source chain; the boundary logs one structured projection.**

```rust
async fn handle_submit(order: Order) -> Result<HttpResponse, StructError<AppReason>> {
    submit_order(order)
        .await
        .source_err(AppReason::system_error(), "handle submit order failed")?;

    Ok(HttpResponse::ok())
}
```

At the handler, worker, or task boundary:

```rust
let report = err.report();
let exposure = err.exposure(&policy);
```

The business code does not need to concatenate log strings at every layer. The boundary can log one structured view containing identity, reason, detail, context, and chain.

### `OperationContext` logging

`OperationContext` provides structured log methods that automatically include current fields and metadata:

```rust
use orion_error::OperationContext;

let ctx = OperationContext::doing("order_processing")
    .with_field("order_id", "123")
    .with_meta("component.name", "order_service");

ctx.info("start");
ctx.warn("slow upstream");
ctx.error("final failure");
```

For lifecycle-scoped logging, use `with_auto_log()`:

```rust
let mut ctx = OperationContext::doing("sync_user")
    .with_auto_log()
    .with_field("user_id", "42");

do_sync()?;
ctx.mark_suc();
```

If the scope drops without `mark_suc()` or `mark_cancel()`, a failure log is emitted automatically. See [LOGGING.md](./LOGGING.md) for details.

The principle: **sparse lifecycle logs + boundary error projection**, not repetitive `error!` at every layer.

---

## 3. Presentation

### One error needs different views for different audiences

An error is consumed by more than one audience:

- End users need safe, understandable, actionable messages.
- Operators / SREs need component, environment, classification, retry, and impact hints.
- Developers need source chain, detail, context, and lower-level cause.
- Protocol clients need stable code, field shape, and retry hints.
- Logs / monitoring / alerting need structured fields, not long strings.

If there is only one error string, it is hard to satisfy all of them well.

For example:

```text
database connection failed: timeout from sqlx pool
```

This is:

- too technical for end users
- too thin for developers
- unstable for protocol clients
- not structured enough for logs

### Recommended approach: keep one structured error, project different views

`orion-error` separates internal structure from external presentation:

- internal: `reason`, `ErrorIdentity.code`, `detail`, `context`, `source chain`
- user-facing: safe and actionable exposure
- operator-facing: component, operation, category, retryability, severity
- developer-facing: report and full chain
- protocol-facing: exposure projection

```rust
let report = err.report();
let exposed = err.exposure(&DefaultExposurePolicy::default());
```

| Audience | Needs | Projection |
|----------|-------|------------|
| User | safe message, action hint | exposure view |
| Operator / SRE | component, operation, retryable, severity | exposure snapshot / log JSON |
| Developer | source chain, detail, context | report |
| Protocol client | stable code, stable fields, retry hint | HTTP/RPC/CLI error JSON |
| Test / regression | stable structure | stable snapshot |

This is the key difference between `orion-error` and a pure display-oriented tool: it keeps one structured error, then projects the right view at the right boundary.

---

## Summary

`orion-error` is for systems where errors are contracts, not just return values.

It helps you:

1. **Preserve failure environment**: attach path, tenant, order id, operation, and component context.
2. **Abstract technical details without losing diagnostics**: convert lower failures into stable layer reasons while preserving source and context.
3. **Keep the cross-layer error chain**: let debugging see how a low-level failure became the final boundary error.
4. **Log effectively and sparingly**: carry structure in the error and log once at the boundary.
5. **Project the right view for each audience**: users, operators, developers, protocol clients, logs, and tests need different output.

In one sentence:

> `orion-error` keeps one structured error across layers, then projects the right view at the right boundary.
