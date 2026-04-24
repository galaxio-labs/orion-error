# 日志说明

`orion-error` 的日志能力围绕 `OperationContext` 和 `OperationScope` 展开。

## 1. Feature

```toml
[dependencies]
orion-error = { version = "0.7.0", features = ["log"] }
# 或
orion-error = { version = "0.7.0", features = ["tracing"] }
```

默认 feature 已包含 `log`。

行为规则：

- 只启用 `log`：使用 `log` 宏输出
- 启用 `tracing`：优先走 `tracing`
- 同时启用：走 `tracing`

## 2. 基本用法

```rust
use orion_error::OperationContext;

let mut ctx = OperationContext::doing("order_processing");
ctx.record_field("order_id", "123");
ctx.record_field("amount", "100.0");
ctx.record_meta("component.name", "order_service");

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

```rust
use orion_error::OperationContext;

let mut ctx = OperationContext::doing("sync_user").with_auto_log();
ctx.record_field("user_id", "42");

do_sync()?;
ctx.mark_suc();
```

默认结果是失败。

如果启用了 `with_auto_log()`，但离开作用域前没有调用：

- `mark_suc()`
- `mark_cancel()`

那么 `Drop` 时会输出失败日志。

兼容旧名：

- `with_exit_log()` 已废弃，当前请使用 `with_auto_log()`

## 4. `OperationScope`

`OperationScope` 是面向一个局部作用域的 guard。

```rust
use orion_error::OperationContext;

let mut ctx = OperationContext::doing("sync_user").with_auto_log();

{
    let mut scope = ctx.scope();
    scope.record_field("user_id", "42");
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

## 5. `scoped_success()` 的使用边界

`scoped_success()` 适合这种场景：

- 作用域里的逻辑已经自行处理完失败分支
- 失败时会明确调用 `mark_failure()`
- 或者这段逻辑本身不会通过 `?` 提前返回

例如：

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

不推荐这样写：

```rust,ignore
let mut scope = ctx.scoped_success();
validate()?;
```

因为当前实现里 `scoped_success()` 一创建就默认成功，如果 `?` 提前返回，`Drop` 仍会把该作用域标记为成功。

对可能早退的 fallible 流程，优先使用：

```rust
let mut scope = ctx.scope();
validate()?;
scope.mark_success();
```

## 6. `op_context!` 宏

```rust
use orion_error::op_context;

let mut ctx = op_context!("load_config").with_auto_log();
ctx.record_field("path", "config.toml");
```

这个宏会在调用点展开 `module_path!()`，让自动结果日志带上更准确的模块路径。

## 7. 推荐实践

- 用 `doing(...)` 命名操作
- 用 `record_field(...)` 记录关键诊断字段
- 用 `record_meta(...)` 记录结构化 metadata
- 用 `with_auto_log()` 只包裹真正需要结果日志的作用域
- 对可能 `?` 提前返回的逻辑，优先 `scope() + mark_success()`
- 只有在失败路径已被显式处理时，再使用 `scoped_success()`
