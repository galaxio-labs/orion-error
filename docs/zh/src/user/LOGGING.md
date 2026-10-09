# 日志说明

`orion-error` 的日志能力围绕三种角色展开：

- `OperationContext` —— **纯数据**：可 `Clone` 的操作字段/元数据/路径/结果载体，**没有 `Drop` 副作用**。
- `AutoLogGuard` —— 由 `OperationContext::with_auto_log()` 创建的所有权型、**不 `Clone` 的 guard**；它在 `Drop` 时**恰好写一次** `suc!` / `fail!` / `cancel!`。
- `OperationScope` —— 借用型 guard，用于标记结果（`mark_success()`）；可作用于 `OperationContext`，也可通过 `AutoLogGuard` 使用。

示例驱动的用法与迁移说明见 [Auto-Log Guard 使用指南](./autolog-guard.md)。

## 1. Feature

```toml
[dependencies]
orion-error = { version = "0.9.0", features = ["log"] }
# 或
orion-error = { version = "0.9.0", features = ["tracing"] }
```

默认 feature 已包含 `log`。

行为规则：

- 只启用 `log`：使用 `log` 宏输出
- 启用 `tracing`：优先走 `tracing`
- 同时启用：走 `tracing`

## 2. 基本用法

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

也可以使用别名：

- `log_info`
- `log_debug`
- `log_warn`
- `log_error`
- `log_trace`

## 3. 自动结果日志

`with_auto_log()` 会消费数据上下文并返回一个 `AutoLogGuard`：

```rust
use orion_error::OperationContext;

let ctx = OperationContext::doing("sync_user").with_field("user_id", "42");
let guard = ctx.with_auto_log();

do_sync()?;
guard.mark_success();
```

guard 在 `Drop` 时写日志，默认结果是失败；若离开作用域前没有调用 `mark_success()` 或 `cancel()`，则输出 `fail!`。

### 数据与 guard 分离

guard 被刻意设计为**不 `Clone`**，因此这个 `Drop` 副作用不可能被复制。若要把同一操作**仅作为数据**附到错误上（例如给 `display_chain()`），借用 guard 即可 —— 只复制数据，不产生任何日志：

```rust
let guard = OperationContext::doing("sync_user").with_auto_log();

let err = build_error().with_context(&guard); // 只复制数据，不打日志
// ... guard 仍是唯一的日志 owner，在自身 Drop 时写一次。
```

刻意**没有**提供按值 `with_context(guard)`：把 guard move 进错误会让日志延迟或重复。若想解除武装并保留纯数据，请用 `guard.into_context()`。

## 4. `OperationScope`

`OperationScope` 用于标记一个借用上下文的结果，可作用于裸的 `OperationContext`，也可通过 `AutoLogGuard`（经 `Deref`）使用。

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

方法：

- `scope()`：默认失败，只有显式 `mark_success()` 才会成功
- `scoped_success()`：创建后默认成功，除非后续显式 `mark_failure()` 或 `cancel()`
- `mark_success()`：标记成功
- `mark_failure()`：恢复为失败
- `cancel()`：标记取消

`AutoLogGuard` 也直接提供 `mark_success()` / `mark_failure()` / `cancel()`（无需再嵌套一层 scope）。

## 5. fallible 流程优先用 `scope()`

`scope()` 默认为失败，因此**与 `?` 天然兼容**：函数提前返回时不会执行 `mark_success()`，失败会被正确记录。

```rust
let mut guard = OperationContext::doing("process_order").with_auto_log();

{
    let mut scope = guard.scope();   // 默认失败
    let value = do_work().await?;    // 提前返回 -> Drop 时 fail!
    scope.mark_success();            // 成功 -> suc!
}
```

`scoped_success()` 一创建就默认成功，因此 `?` 提前返回仍会被记成 `suc!`。只有在每个失败分支都显式调用 `mark_failure()` 时才使用它：

```rust
{
    let mut scope = guard.scoped_success();
    let ok = validate_order();
    if !ok {
        scope.mark_failure();
    }
}
```

## 6. `op_context!` 宏

```rust
use orion_error::op_context;

let guard = op_context!("load_config").with_auto_log().with_field("path", "config.toml");
```

这个宏会在调用点展开 `module_path!()`，让自动结果日志带上更准确的模块路径。

## 7. 推荐实践

- 用 `doing(...)` 命名操作
- 用 `with_field(...)` / `with_meta(...)` 做链式构建
- `record_field(...)` / `record_meta(...)` 只在已有可变引用时使用
- 用 `with_auto_log()` 只包裹真正需要结果日志的作用域
- 对可能 `?` 提前返回的逻辑，优先 `scope() + mark_success()`
- 把上下文附到错误时用 `&guard`（数据副本），让日志始终由 guard 独占
- 只有在失败路径已被显式处理时，再使用 `scoped_success()`
